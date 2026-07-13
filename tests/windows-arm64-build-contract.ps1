[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Assert-Equal {
  param($Actual, $Expected, [string] $Label)

  if ($Actual -ne $Expected) {
    throw "$Label expected '$Expected' but received '$Actual'."
  }
}

function New-PeFixture {
  param([string] $Path, [uint16] $Machine)

  $bytes = New-Object byte[] 128
  $bytes[0] = 0x4d
  $bytes[1] = 0x5a
  [BitConverter]::GetBytes([int] 0x40).CopyTo($bytes, 0x3c)
  $bytes[0x40] = 0x50
  $bytes[0x41] = 0x45
  [BitConverter]::GetBytes($Machine).CopyTo($bytes, 0x44)
  [IO.File]::WriteAllBytes($Path, $bytes)
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$scriptPath = Join-Path $repoRoot "scripts\windows-arm64-build.ps1"
$fixture = Join-Path ([System.IO.Path]::GetTempPath()) "kaydence-arm64-$([Guid]::NewGuid())"

try {
  foreach ($relativePath in @(
    "scripts\windows-webview-proof.ps1",
    "scripts\windows-arm64-build.ps1",
    "tests\windows-arm64-build-contract.ps1"
  )) {
    $tokens = $null
    $errors = $null
    $path = (Resolve-Path (Join-Path $repoRoot $relativePath)).Path
    $null = [Management.Automation.Language.Parser]::ParseFile($path, [ref] $tokens, [ref] $errors)
    if ($errors.Count -gt 0) {
      throw "PowerShell parser rejected $relativePath`: $($errors -join '; ')"
    }
  }

  $help = & $scriptPath -Help | Out-String
  if ($help -notmatch "windows-arm64-build.ps1.*-Check") {
    throw "Help output does not describe the check-mode command."
  }

  $files = @(
    "Common7\Tools\VsDevCmd.bat",
    "Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe",
    "VC\Tools\Llvm\ARM64\bin\clang-cl.exe",
    "VC\Tools\Llvm\ARM64\bin\libclang.dll",
    "VC\Tools\MSVC\14.44.35228\bin\Hostarm64\arm64\link.exe"
  )
  foreach ($relativePath in $files) {
    $path = Join-Path $fixture $relativePath
    $null = New-Item -ItemType Directory -Force -Path (Split-Path $path)
    if ($relativePath -like "*.bat") {
      [IO.File]::WriteAllText($path, "@echo off")
    } else {
      $machine = if ($relativePath -like "*Ninja*") { 0x014c } else { 0xaa64 }
      New-PeFixture $path $machine
    }
  }

  $json = & $scriptPath -BuildToolsPath $fixture -Check | Out-String
  $contract = $json | ConvertFrom-Json
  Assert-Equal $contract.architecture "aarch64-pc-windows-msvc" "architecture"
  Assert-Equal $contract.ggml_native "OFF" "ggml_native"
  Assert-Equal $contract.generate_bindings $true "generate_bindings"
  Assert-Equal $contract.requires_native_powershell $false "PowerShell host requirement"
  Assert-Equal $contract.cargo_locked $true "Cargo lock requirement"
  Assert-Equal $contract.cargo_target "aarch64-pc-windows-msvc" "Cargo target"
  Assert-Equal $contract.required_components.Count 2 "required component count"
  Assert-Equal $contract.build_tools (Resolve-Path $fixture).Path "Build Tools path"
  foreach ($property in @("vsdevcmd", "ninja", "clang", "libclang", "link")) {
    $index = @{ vsdevcmd = 0; ninja = 1; clang = 2; libclang = 3; link = 4 }[$property]
    Assert-Equal $contract.$property (Join-Path $fixture $files[$index]) "$property path"
  }
  Assert-Equal $contract.clang_machine "AA64" "clang machine"
  Assert-Equal $contract.libclang_machine "AA64" "libclang machine"
  Assert-Equal $contract.link_machine "AA64" "link machine"
  Assert-Equal $contract.ninja_machine "x86" "Ninja machine"

  New-PeFixture (Join-Path $fixture $files[2]) 0x8664
  $wrongMachine = try {
    $null = & $scriptPath -BuildToolsPath $fixture -Check
    ""
  } catch { $_.Exception.Message }
  if ($wrongMachine -notmatch "ARM64 clang-cl.*x64") {
    throw "Wrong-machine clang did not fail closed: $wrongMachine"
  }
  New-PeFixture (Join-Path $fixture $files[2]) 0xaa64

  New-PeFixture (Join-Path $fixture $files[1]) 0x0200
  $wrongNinja = try {
    $null = & $scriptPath -BuildToolsPath $fixture -Check
    ""
  } catch { $_.Exception.Message }
  if ($wrongNinja -notmatch "Ninja.*0x0200") {
    throw "Unsupported-machine Ninja did not fail closed: $wrongNinja"
  }
  New-PeFixture (Join-Path $fixture $files[1]) 0x014c

  $priorBindingOverride = [Environment]::GetEnvironmentVariable(
    "WHISPER_DONT_GENERATE_BINDINGS", "Process"
  )
  try {
    [Environment]::SetEnvironmentVariable("WHISPER_DONT_GENERATE_BINDINGS", " ", "Process")
    $bindingFailure = try {
      $null = & $scriptPath -BuildToolsPath $fixture -Check
      ""
    } catch { $_.Exception.Message }
    if ($bindingFailure -notmatch "must be absent") {
      throw "Whitespace binding override did not fail closed: $bindingFailure"
    }
  } finally {
    [Environment]::SetEnvironmentVariable(
      "WHISPER_DONT_GENERATE_BINDINGS", $priorBindingOverride, "Process"
    )
  }

  foreach ($relativePath in $files) {
    $path = Join-Path $fixture $relativePath
    $disabledPath = "$path.missing"
    Move-Item -LiteralPath $path -Destination $disabledPath
    try {
      $failure = try {
        $null = & $scriptPath -BuildToolsPath $fixture -Check
        ""
      } catch { $_.Exception.Message }
      foreach ($component in @(
        "Microsoft.VisualStudio.Component.VC.Llvm.Clang",
        "Microsoft.VisualStudio.Component.VC.Llvm.ClangToolset"
      )) {
        if ($failure -notmatch [regex]::Escape($component)) {
          throw "Missing $relativePath failure did not name $component."
        }
      }
    } finally {
      Move-Item -LiteralPath $disabledPath -Destination $path
    }
  }

  Write-Host "PASS: Windows ARM64 build contract is fail-closed"
} finally {
  Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
}
