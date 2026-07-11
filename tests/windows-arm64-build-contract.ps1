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

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$scriptPath = Join-Path $repoRoot "scripts\windows-arm64-build.ps1"
$fixture = Join-Path ([System.IO.Path]::GetTempPath()) "kaydence-arm64-$([Guid]::NewGuid())"

try {
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
    $null = New-Item -ItemType File -Force -Path $path
  }

  $json = & $scriptPath -BuildToolsPath $fixture -Check | Out-String
  $contract = $json | ConvertFrom-Json
  Assert-Equal $contract.architecture "aarch64-pc-windows-msvc" "architecture"
  Assert-Equal $contract.ggml_native "OFF" "ggml_native"
  Assert-Equal $contract.generate_bindings $true "generate_bindings"
  Assert-Equal $contract.requires_native_powershell $false "PowerShell host requirement"
  Assert-Equal $contract.required_components.Count 2 "required component count"
  Assert-Equal $contract.clang (Join-Path $fixture $files[2]) "clang path"

  Remove-Item -LiteralPath (Join-Path $fixture $files[2])
  $failure = try {
    & $scriptPath -BuildToolsPath $fixture -Check 2>&1 | Out-String
    ""
  } catch {
    $_.Exception.Message
  }
  foreach ($component in @(
    "Microsoft.VisualStudio.Component.VC.Llvm.Clang",
    "Microsoft.VisualStudio.Component.VC.Llvm.ClangToolset"
  )) {
    if ($failure -notmatch [regex]::Escape($component)) {
      throw "Missing-tool failure did not name $component."
    }
  }

  Write-Host "PASS: Windows ARM64 build contract is fail-closed"
} finally {
  Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
}
