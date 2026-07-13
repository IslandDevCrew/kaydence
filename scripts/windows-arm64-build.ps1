[CmdletBinding()]
param(
  [ValidateSet("release", "debug")]
  [string] $Profile = "release",
  [string] $BuildToolsPath = $env:KAYDENCE_VS_BUILD_TOOLS,
  [switch] $Check,
  [Alias("h")]
  [switch] $Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$requiredComponents = @(
  "Microsoft.VisualStudio.Component.VC.Llvm.Clang",
  "Microsoft.VisualStudio.Component.VC.Llvm.ClangToolset"
)
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ($Help) {
  Write-Output "usage: windows-arm64-build.ps1 [-Profile release|debug] [-BuildToolsPath PATH] [-Check] [-Help]"
  return
}

function Stop-Toolchain {
  param([string] $Reason)

  throw "$Reason Required Visual Studio components: $($requiredComponents -join ', ')."
}

function Find-BuildTools {
  param([string] $ExplicitPath)

  if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
    if (-not (Test-Path -LiteralPath $ExplicitPath -PathType Container)) {
      Stop-Toolchain "Build Tools path '$ExplicitPath' does not exist."
    }
    return (Resolve-Path -LiteralPath $ExplicitPath).Path
  }

  $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
  if (Test-Path -LiteralPath $vswhere -PathType Leaf) {
    $arguments = @("-latest", "-products", "*", "-requires") +
      $requiredComponents + @("-property", "installationPath")
    $installation = (& $vswhere @arguments | Select-Object -First 1)
    if (-not [string]::IsNullOrWhiteSpace($installation)) {
      return (Resolve-Path -LiteralPath $installation.Trim()).Path
    }
  }

  foreach ($candidate in @(
    "C:\BuildTools",
    (Join-Path $env:ProgramFiles "Microsoft Visual Studio\2022\BuildTools")
  )) {
    if (Test-Path -LiteralPath $candidate -PathType Container) {
      return (Resolve-Path -LiteralPath $candidate).Path
    }
  }
  Stop-Toolchain "No Visual Studio Build Tools installation with ARM64 LLVM was found."
}

function Require-File {
  param([string] $Path, [string] $Label)

  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    Stop-Toolchain "$Label was not found at '$Path'."
  }
  return (Resolve-Path -LiteralPath $Path).Path
}

function Get-PeMachine {
  param([string] $Path)

  $stream = [IO.File]::OpenRead($Path)
  $reader = New-Object IO.BinaryReader($stream)
  try {
    if ($reader.ReadUInt16() -ne 0x5a4d) { throw "'$Path' has no MZ header." }
    $stream.Position = 0x3c
    $stream.Position = $reader.ReadInt32()
    if ($reader.ReadUInt32() -ne 0x00004550) { throw "'$Path' has no PE header." }
    return $reader.ReadUInt16()
  } finally {
    $reader.Dispose()
    $stream.Dispose()
  }
}

function Get-PeMachineName {
  param([uint16] $Machine)

  switch ($Machine) {
    0x014c { "x86" }
    0x8664 { "x64" }
    0xaa64 { "AA64" }
    default { "0x{0:X4}" -f $Machine }
  }
}

function Require-Arm64Pe {
  param([string] $Path, [string] $Label)

  $machine = Get-PeMachine $Path
  $machineName = Get-PeMachineName $machine
  if ($machine -ne 0xaa64) {
    Stop-Toolchain "$Label at '$Path' is $machineName; expected AA64 (ARM64)."
  }
  return $machineName
}

function Assert-BindingsGenerationEnabled {
  if (Test-Path Env:WHISPER_DONT_GENERATE_BINDINGS) {
    throw "WHISPER_DONT_GENERATE_BINDINGS must be absent so libclang generates target-correct bindings."
  }
}

function Get-ToolchainContract {
  param([string] $Root)

  $llvmBin = Join-Path $Root "VC\Tools\Llvm\ARM64\bin"
  $link = Get-ChildItem -Path (Join-Path $Root "VC\Tools\MSVC\*\bin\Hostarm64\arm64\link.exe") `
    -File -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1
  if ($null -eq $link) {
    Stop-Toolchain "The native ARM64 MSVC linker was not found under '$Root'."
  }

  $vsDevCmd = Require-File (Join-Path $Root "Common7\Tools\VsDevCmd.bat") "VsDevCmd.bat"
  $clang = Require-File (Join-Path $llvmBin "clang-cl.exe") "ARM64 clang-cl"
  $libclang = Require-File (Join-Path $llvmBin "libclang.dll") "ARM64 libclang"
  $ninja = Require-File (Join-Path $Root "Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe") "Ninja"
  $linkPath = $link.FullName
  $ninjaMachine = Get-PeMachineName (Get-PeMachine $ninja)
  if ($ninjaMachine -notin @("x86", "x64", "AA64")) {
    Stop-Toolchain "Ninja at '$ninja' is $ninjaMachine; expected a Windows PE host tool."
  }

  return [ordered]@{
    architecture = "aarch64-pc-windows-msvc"
    build_tools = $Root
    vsdevcmd = $vsDevCmd
    clang = $clang
    libclang = $libclang
    ninja = $ninja
    link = $linkPath
    clang_machine = Require-Arm64Pe $clang "ARM64 clang-cl"
    libclang_machine = Require-Arm64Pe $libclang "ARM64 libclang"
    ninja_machine = $ninjaMachine
    link_machine = Require-Arm64Pe $linkPath "ARM64 linker"
    ggml_native = "OFF"
    generate_bindings = $true
    requires_native_powershell = $false
    cargo_locked = $true
    cargo_target = "aarch64-pc-windows-msvc"
    required_components = $requiredComponents
  }
}

function Import-VsEnvironment {
  param([string] $VsDevCmd)

  $commandFile = Join-Path ([IO.Path]::GetTempPath()) "kaydence-vsdev-$([Guid]::NewGuid()).cmd"
  $commandInterpreter = if ([string]::IsNullOrWhiteSpace($env:ComSpec)) {
    Join-Path ([Environment]::SystemDirectory) "cmd.exe"
  } else {
    $env:ComSpec
  }
  [IO.File]::WriteAllLines($commandFile, @(
    "@echo off",
    "call `"$VsDevCmd`" -no_logo -arch=arm64 -host_arch=arm64 >nul",
    "if errorlevel 1 exit /b %errorlevel%",
    "set"
  ), [Text.Encoding]::ASCII)
  try {
    $lines = & $commandInterpreter /d /s /c "`"$commandFile`""
    if ($LASTEXITCODE -ne 0) {
      throw "VsDevCmd.bat failed with exit code $LASTEXITCODE."
    }
    foreach ($line in $lines) {
      $separator = $line.IndexOf("=")
      if ($separator -gt 0 -and -not $line.StartsWith("=")) {
        [Environment]::SetEnvironmentVariable(
          $line.Substring(0, $separator),
          $line.Substring($separator + 1),
          "Process"
        )
      }
    }
  } finally {
    Remove-Item -LiteralPath $commandFile -Force -ErrorAction SilentlyContinue
  }
}

function Invoke-Checked {
  param([string] $Command, [string[]] $ArgumentList)

  & $Command @ArgumentList
  if ($LASTEXITCODE -ne 0) {
    throw "$Command failed with exit code $LASTEXITCODE."
  }
}

$contract = Get-ToolchainContract (Find-BuildTools $BuildToolsPath)
Assert-BindingsGenerationEnabled
if ($Check) {
  $contract | ConvertTo-Json -Depth 3
  return
}

Import-VsEnvironment $contract.vsdevcmd
Assert-BindingsGenerationEnabled
$llvmBin = Split-Path $contract.clang
$env:Path = "$llvmBin;$(Split-Path $contract.ninja);$(Split-Path $contract.link);$env:Path"
$env:CMAKE_GENERATOR = "Ninja"
$env:CMAKE_C_COMPILER = $contract.clang
$env:CMAKE_CXX_COMPILER = $contract.clang
$env:CMAKE_ASM_COMPILER = $contract.clang
$env:CXXFLAGS = "/EHsc"
$env:LIBCLANG_PATH = $llvmBin
$env:GGML_NATIVE = "OFF"

$clangVersion = (& $contract.clang --version | Out-String)
$clangTarget = [regex]::Match($clangVersion, "(?m)^Target:\s+(\S+)")
if ($LASTEXITCODE -ne 0 -or -not $clangTarget.Success -or
    $clangTarget.Groups[1].Value -ne $contract.architecture) {
  throw "clang-cl target is not $($contract.architecture). Output: $clangVersion"
}

$rustHost = (& rustc -vV | Select-String "^host:").Line.Split(":", 2)[1].Trim()
if ($LASTEXITCODE -ne 0 -or $rustHost -ne $contract.architecture) {
  throw "Rust host '$rustHost' is not $($contract.architecture)."
}

$manifest = Join-Path $repoRoot "apps\desktop\src-tauri\Cargo.toml"
$profileArguments = if ($Profile -eq "release") { @("--release") } else { @() }
$targetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  Join-Path $repoRoot "target"
} elseif ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
  $env:CARGO_TARGET_DIR
} else {
  Join-Path $repoRoot $env:CARGO_TARGET_DIR
}
$binary = Join-Path $targetRoot "$($contract.cargo_target)\$Profile\kaydence.exe"
Remove-Item -LiteralPath $binary -Force -ErrorAction SilentlyContinue
Push-Location $repoRoot
try {
  Invoke-Checked "pnpm" @("--filter", "kaydence-desktop", "build")
  Invoke-Checked "cargo" (@("clean") + $profileArguments + @(
    "--target", $contract.cargo_target, "--manifest-path", $manifest, "-p", "whisper-rs-sys"
  ))
  Assert-BindingsGenerationEnabled
  Invoke-Checked "cargo" (@("build") + $profileArguments + @(
    "--locked", "--target", $contract.cargo_target,
    "--features", "custom-protocol,asr-whisper", "--manifest-path", $manifest
  ))
} finally {
  Pop-Location
}

if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
  throw "Cargo succeeded but Kaydence was not found at '$binary'."
}
if ((Get-PeMachine $binary) -ne 0xaa64) {
  throw "Kaydence output at '$binary' is not an AA64 (ARM64) PE executable."
}
Write-Host "Kaydence Windows ARM64 build ready: $binary"
