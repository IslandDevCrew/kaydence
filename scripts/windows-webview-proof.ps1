[CmdletBinding()]
param(
  [string] $ExecutablePath,
  [string] $OutputDirectory,
  [ValidateRange(15, 300)]
  [int] $TimeoutSeconds = 90
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ([string]::IsNullOrWhiteSpace($ExecutablePath)) {
  $ExecutablePath = Join-Path $repoRoot "target\release\kaydence.exe"
}
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
  $OutputDirectory = Join-Path $repoRoot "ops\mission\evidence\windows-webview-proof"
}

$ExecutablePath = [System.IO.Path]::GetFullPath($ExecutablePath)
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
$null = New-Item -ItemType Directory -Force -Path $OutputDirectory

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class NativeWindowProof {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct POINT {
        public int X;
        public int Y;
    }

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool ClientToScreen(IntPtr hWnd, ref POINT lpPoint);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool MoveWindow(
        IntPtr hWnd,
        int X,
        int Y,
        int nWidth,
        int nHeight,
        bool bRepaint
    );

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool ShowWindowAsync(IntPtr hWnd, int nCmdShow);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
}
"@

function Get-SourceHashes {
  $hashes = [ordered]@{}
  $sourceFiles = @(
    "apps/desktop/src/App.tsx",
    "apps/desktop/src/views/DictateView.tsx",
    "apps/desktop/src/views/CleanupView.tsx",
    "apps/desktop/src/views/PrivacyView.tsx",
    "apps/desktop/src/views/FirstRunView.tsx"
  )

  foreach ($relativePath in $sourceFiles) {
    $path = Join-Path $repoRoot $relativePath
    if (Test-Path -LiteralPath $path) {
      $hashes[$relativePath] = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
    }
  }
  return $hashes
}

function Get-HostFacts {
  $os = $null
  $computer = $null
  try { $os = Get-CimInstance Win32_OperatingSystem } catch { }
  try { $computer = Get-CimInstance Win32_ComputerSystem } catch { }

  $osCaption = if ($null -ne $os) { $os.Caption } else { [System.Environment]::OSVersion.VersionString }
  $osVersion = if ($null -ne $os) { $os.Version } else { [System.Environment]::OSVersion.Version.ToString() }
  $osBuild = if ($null -ne $os) { $os.BuildNumber } else { $null }
  $manufacturer = if ($null -ne $computer) { $computer.Manufacturer } else { $null }
  $model = if ($null -ne $computer) { $computer.Model } else { $null }

  return [ordered]@{
    os_caption = $osCaption
    os_version = $osVersion
    os_build = $osBuild
    architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    manufacturer = $manufacturer
    model = $model
    user_interactive = [System.Environment]::UserInteractive
    session_id = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
    session_name = $env:SESSIONNAME
    runner_name = $env:RUNNER_NAME
    runner_os = $env:RUNNER_OS
    runner_arch = $env:RUNNER_ARCH
    runner_image = $env:ImageOS
  }
}

function Wait-NativeWindow {
  param(
    [System.Diagnostics.Process] $Process,
    [int] $Timeout
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($Timeout)
  while ([DateTime]::UtcNow -lt $deadline) {
    if ($Process.HasExited) {
      throw "The desktop process exited before exposing a native window (exit $($Process.ExitCode))."
    }
    $Process.Refresh()
    if ($Process.MainWindowHandle -ne [IntPtr]::Zero) {
      return $Process.MainWindowHandle
    }
    Start-Sleep -Milliseconds 250
  }
  throw "Timed out after $Timeout seconds waiting for the native desktop window."
}

function Get-WindowMetrics {
  param([IntPtr] $WindowHandle)

  $client = [NativeWindowProof+RECT]::new()
  $window = [NativeWindowProof+RECT]::new()
  if (-not [NativeWindowProof]::GetClientRect($WindowHandle, [ref] $client)) {
    throw "GetClientRect failed with Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())."
  }
  if (-not [NativeWindowProof]::GetWindowRect($WindowHandle, [ref] $window)) {
    throw "GetWindowRect failed with Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())."
  }

  return [pscustomobject]@{
    ClientWidth = $client.Right - $client.Left
    ClientHeight = $client.Bottom - $client.Top
    WindowX = $window.Left
    WindowY = $window.Top
    WindowWidth = $window.Right - $window.Left
    WindowHeight = $window.Bottom - $window.Top
  }
}

function Resize-NativeClient {
  param(
    [IntPtr] $WindowHandle,
    [int] $Width,
    [int] $Height
  )

  for ($attempt = 0; $attempt -lt 3; $attempt += 1) {
    $metrics = Get-WindowMetrics -WindowHandle $WindowHandle
    $frameWidth = $metrics.WindowWidth - $metrics.ClientWidth
    $frameHeight = $metrics.WindowHeight - $metrics.ClientHeight
    $outerWidth = $Width + $frameWidth
    $outerHeight = $Height + $frameHeight
    if (-not [NativeWindowProof]::MoveWindow($WindowHandle, 32, 32, $outerWidth, $outerHeight, $true)) {
      throw "MoveWindow failed with Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())."
    }
    Start-Sleep -Milliseconds 300
    $resized = Get-WindowMetrics -WindowHandle $WindowHandle
    if ($resized.ClientWidth -eq $Width -and $resized.ClientHeight -eq $Height) {
      return $resized
    }
  }

  $final = Get-WindowMetrics -WindowHandle $WindowHandle
  throw "Native client is $($final.ClientWidth)x$($final.ClientHeight); expected ${Width}x${Height}."
}

function Wait-UiaElement {
  param(
    [IntPtr] $WindowHandle,
    [string] $Name,
    [string] $ControlTypeName = "",
    [int] $Timeout = $TimeoutSeconds
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($Timeout)
  $condition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::NameProperty,
    $Name
  )

  while ([DateTime]::UtcNow -lt $deadline) {
    try {
      $root = [System.Windows.Automation.AutomationElement]::FromHandle($WindowHandle)
      $matches = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
      for ($index = 0; $index -lt $matches.Count; $index += 1) {
        $candidate = $matches.Item($index)
        $programmaticName = $candidate.Current.ControlType.ProgrammaticName
        if ([string]::IsNullOrWhiteSpace($ControlTypeName) -or $programmaticName -eq "ControlType.$ControlTypeName") {
          return $candidate
        }
      }
    } catch [System.Windows.Automation.ElementNotAvailableException] {
      # WebView navigation can briefly replace the accessibility subtree.
    }
    Start-Sleep -Milliseconds 250
  }

  $typeDetail = if ([string]::IsNullOrWhiteSpace($ControlTypeName)) { "element" } else { $ControlTypeName }
  throw "Timed out waiting for UI Automation $typeDetail named '$Name'."
}

function Invoke-UiaButton {
  param(
    [IntPtr] $WindowHandle,
    [string] $Name
  )

  $button = Wait-UiaElement -WindowHandle $WindowHandle -Name $Name -ControlTypeName "Button"
  $pattern = $null
  if ($button.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref] $pattern)) {
    ([System.Windows.Automation.InvokePattern] $pattern).Invoke()
    return
  }

  $legacyPattern = $null
  if ($button.TryGetCurrentPattern([System.Windows.Automation.LegacyIAccessiblePattern]::Pattern, [ref] $legacyPattern)) {
    ([System.Windows.Automation.LegacyIAccessiblePattern] $legacyPattern).DoDefaultAction()
    return
  }

  throw "UI Automation button '$Name' exposes neither Invoke nor default-action support."
}

function Export-UiaTree {
  param(
    [IntPtr] $WindowHandle,
    [string] $Path
  )

  $root = [System.Windows.Automation.AutomationElement]::FromHandle($WindowHandle)
  $elements = $root.FindAll(
    [System.Windows.Automation.TreeScope]::Descendants,
    [System.Windows.Automation.Condition]::TrueCondition
  )
  $lines = [System.Collections.Generic.List[string]]::new()
  $null = $lines.Add("index`tcontrol_type`tname`tautomation_id`tenabled")
  $limit = [Math]::Min($elements.Count, 2000)
  for ($index = 0; $index -lt $limit; $index += 1) {
    try {
      $element = $elements.Item($index)
      $name = ($element.Current.Name -replace "[`r`n`t]+", " ").Trim()
      $automationId = ($element.Current.AutomationId -replace "[`r`n`t]+", " ").Trim()
      $null = $lines.Add(
        "$index`t$($element.Current.ControlType.ProgrammaticName)`t$name`t$automationId`t$($element.Current.IsEnabled)"
      )
    } catch [System.Windows.Automation.ElementNotAvailableException] {
      $null = $lines.Add("$index`tElementNotAvailable")
    }
  }
  $lines | Set-Content -LiteralPath $Path -Encoding utf8
}

function Test-CapturedBitmap {
  param(
    [System.Drawing.Bitmap] $Bitmap,
    [int] $ExpectedWidth = 0,
    [int] $ExpectedHeight = 0
  )

  if ($ExpectedWidth -gt 0 -and $Bitmap.Width -ne $ExpectedWidth) {
    throw "Captured bitmap width is $($Bitmap.Width); expected $ExpectedWidth."
  }
  if ($ExpectedHeight -gt 0 -and $Bitmap.Height -ne $ExpectedHeight) {
    throw "Captured bitmap height is $($Bitmap.Height); expected $ExpectedHeight."
  }

  $colors = [System.Collections.Generic.HashSet[int]]::new()
  $minimumLuma = 255
  $maximumLuma = 0
  $sampleCount = 0
  $nonDarkCount = 0
  $stepX = [Math]::Max(1, [int] [Math]::Floor($Bitmap.Width / 75))
  $stepY = [Math]::Max(1, [int] [Math]::Floor($Bitmap.Height / 50))

  for ($y = 0; $y -lt $Bitmap.Height; $y += $stepY) {
    for ($x = 0; $x -lt $Bitmap.Width; $x += $stepX) {
      $color = $Bitmap.GetPixel($x, $y)
      $packed = ($color.R -shl 16) -bor ($color.G -shl 8) -bor $color.B
      $null = $colors.Add($packed)
      $luma = [int] (($color.R * 299 + $color.G * 587 + $color.B * 114) / 1000)
      $minimumLuma = [Math]::Min($minimumLuma, $luma)
      $maximumLuma = [Math]::Max($maximumLuma, $luma)
      if ($luma -gt 8) { $nonDarkCount += 1 }
      $sampleCount += 1
    }
  }

  $nonDarkRatio = if ($sampleCount -gt 0) { $nonDarkCount / $sampleCount } else { 0 }
  if ($colors.Count -lt 24 -or ($maximumLuma - $minimumLuma) -lt 20 -or $nonDarkRatio -lt 0.10) {
    throw "Capture appears blank or uniform (colors=$($colors.Count), luma=${minimumLuma}-${maximumLuma}, non_dark=$([Math]::Round($nonDarkRatio, 3)))."
  }

  return [ordered]@{
    width = $Bitmap.Width
    height = $Bitmap.Height
    sampled_colors = $colors.Count
    minimum_luma = $minimumLuma
    maximum_luma = $maximumLuma
    non_dark_ratio = [Math]::Round($nonDarkRatio, 4)
  }
}

function Capture-Region {
  param(
    [int] $X,
    [int] $Y,
    [int] $Width,
    [int] $Height,
    [string] $Path,
    [int] $ExpectedWidth = 0,
    [int] $ExpectedHeight = 0
  )

  $bitmap = [System.Drawing.Bitmap]::new(
    $Width,
    $Height,
    [System.Drawing.Imaging.PixelFormat]::Format24bppRgb
  )
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  try {
    $graphics.CopyFromScreen(
      $X,
      $Y,
      0,
      0,
      [System.Drawing.Size]::new($Width, $Height),
      [System.Drawing.CopyPixelOperation]::SourceCopy
    )
  } finally {
    $graphics.Dispose()
  }

  try {
    $validation = Test-CapturedBitmap -Bitmap $bitmap -ExpectedWidth $ExpectedWidth -ExpectedHeight $ExpectedHeight
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $bitmap.Dispose()
  }

  $validation["file"] = [System.IO.Path]::GetFileName($Path)
  $validation["sha256"] = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
  return $validation
}

function Capture-AppState {
  param(
    [IntPtr] $WindowHandle,
    [string] $Slug,
    [string] $ExpectedName
  )

  $null = Wait-UiaElement -WindowHandle $WindowHandle -Name $ExpectedName
  $null = [NativeWindowProof]::ShowWindowAsync($WindowHandle, 9)
  $null = [NativeWindowProof]::SetForegroundWindow($WindowHandle)
  Start-Sleep -Milliseconds 600

  $metrics = Get-WindowMetrics -WindowHandle $WindowHandle
  if ($metrics.ClientWidth -ne 900 -or $metrics.ClientHeight -ne 600) {
    throw "Client changed to $($metrics.ClientWidth)x$($metrics.ClientHeight) before '$Slug' capture."
  }

  $clientOrigin = [NativeWindowProof+POINT]::new()
  $clientOrigin.X = 0
  $clientOrigin.Y = 0
  if (-not [NativeWindowProof]::ClientToScreen($WindowHandle, [ref] $clientOrigin)) {
    throw "ClientToScreen failed with Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())."
  }

  $clientPath = Join-Path $OutputDirectory "$Slug-native-windows.png"
  $windowPath = Join-Path $OutputDirectory "$Slug-native-windows-runner.png"
  $uiaPath = Join-Path $OutputDirectory "$Slug-native-windows-uia.tsv"
  $clientCapture = Capture-Region `
    -X $clientOrigin.X `
    -Y $clientOrigin.Y `
    -Width $metrics.ClientWidth `
    -Height $metrics.ClientHeight `
    -Path $clientPath `
    -ExpectedWidth 900 `
    -ExpectedHeight 600
  $windowCapture = Capture-Region `
    -X $metrics.WindowX `
    -Y $metrics.WindowY `
    -Width $metrics.WindowWidth `
    -Height $metrics.WindowHeight `
    -Path $windowPath
  Export-UiaTree -WindowHandle $WindowHandle -Path $uiaPath

  return [ordered]@{
    slug = $Slug
    expected_uia_name = $ExpectedName
    client = $clientCapture
    native_window = $windowCapture
    uia_tree = [System.IO.Path]::GetFileName($uiaPath)
    uia_tree_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $uiaPath).Hash.ToLowerInvariant()
  }
}

$appProcess = $null
$windowHandle = [IntPtr]::Zero
$captures = [System.Collections.Generic.List[object]]::new()
$failure = $null
$startedAt = [DateTime]::UtcNow

try {
  if (-not [System.Environment]::UserInteractive) {
    throw "The runner does not expose an interactive Windows session. Native WebView proof cannot be claimed."
  }
  if (-not (Test-Path -LiteralPath $ExecutablePath)) {
    throw "Release executable not found: $ExecutablePath"
  }

  try {
    $null = [NativeWindowProof]::SetProcessDpiAwarenessContext([IntPtr]::new(-4))
  } catch {
    Write-Host "DPI awareness context was already fixed by the runner host; continuing with measured client bounds."
  }

  $appProcess = Start-Process -FilePath $ExecutablePath -PassThru
  $windowHandle = Wait-NativeWindow -Process $appProcess -Timeout $TimeoutSeconds
  $null = Resize-NativeClient -WindowHandle $windowHandle -Width 900 -Height 600

  $null = Wait-UiaElement -WindowHandle $windowHandle -Name "Recent clean transcript"
  $captures.Add((Capture-AppState -WindowHandle $windowHandle -Slug "board01" -ExpectedName "Recent clean transcript"))

  Invoke-UiaButton -WindowHandle $windowHandle -Name "Cleanup"
  $null = Wait-UiaElement -WindowHandle $windowHandle -Name "Cleanup & Injection"
  $captures.Add((Capture-AppState -WindowHandle $windowHandle -Slug "board03" -ExpectedName "Cleanup & Injection"))

  Invoke-UiaButton -WindowHandle $windowHandle -Name "Privacy"
  $null = Wait-UiaElement -WindowHandle $windowHandle -Name "Privacy & Context"
  $captures.Add((Capture-AppState -WindowHandle $windowHandle -Slug "board07" -ExpectedName "Privacy & Context"))

  Invoke-UiaButton -WindowHandle $windowHandle -Name "General"
  $null = Wait-UiaElement -WindowHandle $windowHandle -Name "First Run Setup & License"
  $captures.Add((Capture-AppState -WindowHandle $windowHandle -Slug "board10" -ExpectedName "First Run Setup & License"))

  Invoke-UiaButton -WindowHandle $windowHandle -Name "Review"
  $null = Wait-UiaElement -WindowHandle $windowHandle -Name "First Run Proof"
  $captures.Add((Capture-AppState -WindowHandle $windowHandle -Slug "board10-permissions" -ExpectedName "First Run Proof"))
  foreach ($permissionName in @("Microphone", "UI Automation focus access", "Keyboard injection fallback")) {
    $null = Wait-UiaElement -WindowHandle $windowHandle -Name $permissionName
  }

  $manifest = [ordered]@{
    schema_version = 1
    result = "pass"
    captured_at_utc = [DateTime]::UtcNow.ToString("o")
    elapsed_seconds = [Math]::Round(([DateTime]::UtcNow - $startedAt).TotalSeconds, 3)
    commit = $env:GITHUB_SHA
    run_id = $env:GITHUB_RUN_ID
    run_attempt = $env:GITHUB_RUN_ATTEMPT
    workflow = $env:GITHUB_WORKFLOW
    executable = [System.IO.Path]::GetFileName($ExecutablePath)
    executable_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $ExecutablePath).Hash.ToLowerInvariant()
    host = (Get-HostFacts)
    source_hashes = (Get-SourceHashes)
    captures = $captures
    boundary = "This proves native Windows Tauri/WebView rendering and UI Automation navigation only. It does not prove microphone, global hotkey, text injection, secure-field refusal, ASR, or first-dictation readiness."
  }
  $manifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $OutputDirectory "manifest.json") -Encoding utf8
  Write-Host "Windows native WebView proof passed: $($captures.Count) states captured in $OutputDirectory"
} catch {
  $failure = [ordered]@{
    schema_version = 1
    result = "fail"
    failed_at_utc = [DateTime]::UtcNow.ToString("o")
    elapsed_seconds = [Math]::Round(([DateTime]::UtcNow - $startedAt).TotalSeconds, 3)
    commit = $env:GITHUB_SHA
    run_id = $env:GITHUB_RUN_ID
    executable = $ExecutablePath
    host = (Get-HostFacts)
    source_hashes = (Get-SourceHashes)
    completed_captures = $captures
    error = $_.Exception.Message
    script_stack = $_.ScriptStackTrace
    boundary = "Failure is fail-closed. No Windows WebView proof may be claimed from this run."
  }
  $failure | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $OutputDirectory "failure.json") -Encoding utf8
} finally {
  if ($null -ne $appProcess -and -not $appProcess.HasExited) {
    Stop-Process -Id $appProcess.Id -Force -ErrorAction SilentlyContinue
    $appProcess.WaitForExit(5000) | Out-Null
  }
}

if ($null -ne $failure) {
  Write-Host "::error::$($failure.error)"
  exit 1
}
