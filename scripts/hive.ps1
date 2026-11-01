<#
.SYNOPSIS
    H.I.V.E. Autonomous AI Agency CLI Tool
    Controls headless rendering, screenshots, test execution, and scene management.

.DESCRIPTION
    Provides automated CLI commands for H.I.V.E. developers:
    - capture   : Takes headless screenshots of 3D cybernetic scenes and workspaces
    - test      : Runs the comprehensive end-to-end test suite in batchmode
    - scenes    : Re-generates or verifies all 7 operational scenes and prefabs
    - doctor    : Validates Unity environment, agent schemas, licenses, and dependencies

.EXAMPLE
    .\scripts\hive.ps1 capture --all --res 1920x1080
    .\scripts\hive.ps1 capture --scene CommandChamber --res 3840x2160 --out ./stills
    .\scripts\hive.ps1 test
    .\scripts\hive.ps1 doctor
#>

param(
    [string]$Command = "help"
)

$RemainingArgs = $args

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

# Locate Unity executable
$UnityPaths = @(
    "C:\Program Files\Unity\Hub\Editor\6000.6.7f1\Editor\Unity.exe",
    "C:\Program Files\Unity\Editor\Unity.exe"
)

$UnityExe = $null
foreach ($path in $UnityPaths) {
    if (Test-Path $path) {
        $UnityExe = $path
        break
    }
}

if (-not $UnityExe) {
    $found = Get-ChildItem -Path "C:\Program Files\Unity\Hub\Editor" -Filter "Unity.exe" -Recurse -Depth 3 -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) { $UnityExe = $found.FullName }
}

function Show-Header {
    Write-Host "==================================================================" -ForegroundColor Green
    Write-Host "  ⬡ H.I.V.E. // AUTONOMOUS AI AGENCY CLI TOOL" -ForegroundColor Green
    Write-Host "==================================================================" -ForegroundColor DarkGreen
}

function Invoke-Capture {
    param([string[]]$ArgsList)

    $scene = "all"
    $res = "1920x1080"
    $outDir = "Screenshots"
    $mode = ""

    for ($i = 0; $i -lt $ArgsList.Length; $i++) {
        $a = $ArgsList[$i]
        if ($a -eq "--all" -or $a -eq "-all") { $scene = "all" }
        elseif ($a -eq "--scene" -or $a -eq "-s") { $scene = $ArgsList[++$i] }
        elseif ($a -eq "--res" -or $a -eq "-r") { $res = $ArgsList[++$i] }
        elseif ($a -eq "--out" -or $a -eq "-o") { $outDir = $ArgsList[++$i] }
        elseif ($a -eq "--mode" -or $a -eq "-m") { $mode = $ArgsList[++$i] }
    }

    Write-Host "[HIVE CLI] Dispatching Headless Screenshot Capture..." -ForegroundColor Cyan
    Write-Host "  Scene:      $scene" -ForegroundColor Gray
    Write-Host "  Resolution: $res" -ForegroundColor Gray
    Write-Host "  Output Dir: $outDir" -ForegroundColor Gray
    if ($mode) { Write-Host "  CameraMode: $mode" -ForegroundColor Gray }

    $targetDir = Join-Path $ProjectRoot $outDir
    if (-not (Test-Path $targetDir)) {
        New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
    }

    $logFile = Join-Path $ProjectRoot "capture.log"
    $unityArgs = @(
        "-batchmode",
        "-quit",
        "-projectPath", "`"$ProjectRoot`"",
        "-executeMethod", "HIVE.Editor.HiveScreenshotCapture.CaptureCLI",
        "-scene", $scene,
        "-res", $res,
        "-out", "`"$targetDir`""
    )
    if ($mode) {
        $unityArgs += @("-cameraMode", $mode)
    }
    $unityArgs += @("-logFile", "`"$logFile`"")

    $argString = $unityArgs -join " "
    Write-Host "[HIVE CLI] Launching Unity batchmode renderer..." -ForegroundColor DarkCyan

    $process = Start-Process -FilePath $UnityExe -ArgumentList $argString -PassThru -Wait
    $exitCode = $process.ExitCode

    if (Test-Path $logFile) {
        $captures = Get-Content $logFile | Select-String "\[OK\] Captured"
        foreach ($c in $captures) {
            Write-Host "  $c" -ForegroundColor Green
        }
    }

    if ($exitCode -eq 0) {
        Write-Host "[HIVE CLI] SUCCESS: All requested screenshots captured!" -ForegroundColor Green
        $files = Get-ChildItem -Path $targetDir -Filter "*.png" | Sort-Object LastWriteTime -Descending | Select-Object -First 7
        Write-Host "`nRecent captures in $outDir`:" -ForegroundColor Cyan
        foreach ($f in $files) {
            Write-Host "  - $($f.Name) ($([math]::Round($f.Length / 1KB, 1)) KB)" -ForegroundColor Gray
        }
    } else {
        Write-Host "[HIVE CLI] ERROR: Capture failed with exit code $exitCode. See capture.log for details." -ForegroundColor Red
    }
}

function Invoke-Test {
    Write-Host "[HIVE CLI] Running Comprehensive E2E Validation Suite..." -ForegroundColor Cyan
    $logFile = Join-Path $ProjectRoot "unity_e2e.log"
    $unityArgs = "-batchmode -quit -projectPath `"$ProjectRoot`" -executeMethod HIVE.Editor.HiveE2ETest.RunAllTests -logFile `"$logFile`""

    $process = Start-Process -FilePath $UnityExe -ArgumentList $unityArgs -PassThru -Wait
    $exitCode = $process.ExitCode

    if (Test-Path $logFile) {
        Get-Content $logFile | Select-String "\[HIVE E2E\]" | ForEach-Object {
            if ($_ -match "\[PASS\]") {
                Write-Host "  $_" -ForegroundColor Green
            } elseif ($_ -match "\[FAIL\]") {
                Write-Host "  $_" -ForegroundColor Red
            } else {
                Write-Host "  $_" -ForegroundColor Cyan
            }
        }
    }

    if ($exitCode -eq 0) {
        Write-Host "[HIVE CLI] ALL SUITES PASSED! System operational." -ForegroundColor Green
    } else {
        Write-Host "[HIVE CLI] SUITE FAILED with exit code $exitCode." -ForegroundColor Red
    }
}

function Invoke-Scenes {
    Write-Host "[HIVE CLI] Rebuilding all 7 operational HQ scenes and prefabs..." -ForegroundColor Cyan
    $logFile = Join-Path $ProjectRoot "build_scenes.log"
    $unityArgs = "-batchmode -quit -projectPath `"$ProjectRoot`" -executeMethod HIVE.Editor.HiveSceneBuilder.BuildAll -logFile `"$logFile`""

    $process = Start-Process -FilePath $UnityExe -ArgumentList $unityArgs -PassThru -Wait
    if ($process.ExitCode -eq 0) {
        Write-Host "[HIVE CLI] All 7 scenes and prefabs rebuilt successfully." -ForegroundColor Green
    } else {
        Write-Host "[HIVE CLI] Scene build failed. See build_scenes.log." -ForegroundColor Red
    }
}

function Invoke-Build {
    Write-Host "[HIVE CLI] Compiling Standalone Windows 64-bit Binary..." -ForegroundColor Cyan
    $buildsDir = Join-Path $ProjectRoot "Builds"
    if (-not (Test-Path $buildsDir)) {
        New-Item -ItemType Directory -Force -Path $buildsDir | Out-Null
    }
    $targetExe = Join-Path $buildsDir "HIVE.exe"
    $logFile = Join-Path $ProjectRoot "build_player.log"
    $unityArgs = "-batchmode -quit -projectPath `"$ProjectRoot`" -executeMethod HIVE.Editor.HiveSceneBuilder.BuildStandalonePlayer -logFile `"$logFile`""

    Write-Host "  Target:  $targetExe" -ForegroundColor Gray
    Write-Host "  Compiling operational scenes, assets, and cybernetic runtime..." -ForegroundColor DarkCyan

    $process = Start-Process -FilePath $UnityExe -ArgumentList $unityArgs -PassThru -Wait
    if ($process.ExitCode -eq 0 -and (Test-Path $targetExe)) {
        $sizeMb = [math]::Round((Get-Item $targetExe).Length / 1MB, 2)
        Write-Host "[HIVE CLI] SUCCESS: Standalone Windows Binary built! ($sizeMb MB)" -ForegroundColor Green
        Write-Host "Execute directly via: .\Builds\HIVE.exe or hive launch" -ForegroundColor Cyan
    } else {
        Write-Host "[HIVE CLI] ERROR: Player build failed with code $($process.ExitCode). See build_player.log." -ForegroundColor Red
    }
}

function Invoke-Doctor {
    Write-Host "[HIVE CLI] Running Agency Health Doctor..." -ForegroundColor Cyan

    # Unity check
    if ($UnityExe -and (Test-Path $UnityExe)) {
        Write-Host "  [OK] Unity Editor found: $UnityExe" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Unity Editor not located." -ForegroundColor Red
    }

    # Agent JSONs
    $agentsDir = Join-Path $ProjectRoot "Assets\StreamingAssets\agents"
    $agents = Get-ChildItem -Path $agentsDir -Filter "*.json" -ErrorAction SilentlyContinue
    if ($agents.Count -eq 12) {
        Write-Host "  [OK] 12 Persistent Agent Schemas verified in StreamingAssets." -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Expected 12 agents, found $($agents.Count)." -ForegroundColor Red
    }

    # Scenes
    $scenesDir = Join-Path $ProjectRoot "Assets\Scenes"
    $scenes = Get-ChildItem -Path $scenesDir -Filter "*.unity" -ErrorAction SilentlyContinue
    if ($scenes.Count -ge 7) {
        Write-Host "  [OK] 7 Operational Scenes verified ($($scenes.Count) found)." -ForegroundColor Green
    } else {
        Write-Host "  [WARN] Expected 7 scenes, found $($scenes.Count). Run 'hive scenes' to rebuild." -ForegroundColor Yellow
    }

    # NIM API Key check
    $nimKey = [Environment]::GetEnvironmentVariable("NVIDIA_API_KEY", "User")
    if ($nimKey) {
        Write-Host "  [OK] NVIDIA_API_KEY detected in User environment." -ForegroundColor Green
    } else {
        Write-Host "  [WARN] NVIDIA_API_KEY not set. Falling back to offline/mock mode." -ForegroundColor Yellow
    }

    # Google Ecosystem / gog CLI check
    $userProf = [Environment]::GetFolderPath("UserProfile")
    $gogBat = Join-Path $userProf "gog-wrapper\gog.bat"
    $gogExe = Join-Path $userProf "go\bin\gog.exe"
    if (Test-Path $gogBat) {
        Write-Host "  [OK] Google CLI wrapper detected at $gogBat" -ForegroundColor Green
        try {
            $authJson = & "$gogBat" auth status --json 2>$null | Out-String | ConvertFrom-Json
            if ($authJson -and $authJson.account.email) {
                Write-Host "  [OK] Google Account authenticated: $($authJson.account.email)" -ForegroundColor Green
            }
        } catch { }
    } elseif (Test-Path $gogExe) {
        Write-Host "  [OK] Google CLI detected at $gogExe" -ForegroundColor Green
    } else {
        Write-Host "  [WARN] Google CLI (gog) not detected in standard paths." -ForegroundColor Yellow
    }
}

function Show-Help {
    Show-Header
    Write-Host @"
USAGE:
  hive <command> [options]

COMMANDS:
  launch           Launch interactive H.I.V.E. Unity Editor & Command Chamber
  capture          Take headless 3D screenshots of operational scenes
  test             Execute comprehensive end-to-end integration test suite
  scenes           Rebuild and wire all 7 HQ scenes and cybernetic prefabs
  doctor           Inspect health of Unity, schemas, scenes, and API keys
  gog <args>       Run Google Ecosystem CLI directly with authentication
  help             Show this help reference

CAPTURE OPTIONS:
  --all            Capture all 7 operational scenes (default: all)
  --scene <name>   Capture a specific scene (CommandChamber, Workspaces, etc.)
  --res <WxH>      Render resolution (e.g. 1920x1080, 3840x2160, 1280x720)
  --out <dir>      Output directory for screenshots (default: Screenshots)
  --mode <mode>    Camera mode: FirstPerson, ThirdPerson, Isometric, Overview

EXAMPLES:
  .\hive.cmd launch
  .\hive.cmd capture
  .\hive.cmd capture --scene CommandChamber --res 3840x2160 --out stills
  .\hive.cmd test
  .\hive.cmd doctor
"@
}

function Invoke-Launch {
    Write-Host "[HIVE CLI] Initiating Agency Launch Protocol..." -ForegroundColor Cyan
    
    # 1. Run recovery watchdog check
    Write-Host "  [1/3] Verifying Recovery Supervisor..." -ForegroundColor Gray
    try {
        & py "$ProjectRoot\scripts\recovery\hive_recovery.py" watchdog
    } catch {
        Write-Host "  [WARN] Watchdog check encountered an issue." -ForegroundColor Yellow
    }

    # 2. Check binary
    $targetExe = Join-Path $ProjectRoot "Builds\HIVE.exe"
    if (-not (Test-Path $targetExe)) {
        Write-Host "  [2/3] Standalone player not found. Compiling native executable..." -ForegroundColor Yellow
        Invoke-Build
    } else {
        Write-Host "  [2/3] Standalone HIVE binary verified." -ForegroundColor Gray
    }

    # 3. Launch
    if (Test-Path $targetExe) {
        Write-Host "  [3/3] Launching standalone H.I.V.E. application..." -ForegroundColor Green
        $buildsDir = Split-Path -Parent $targetExe
        $proc = Start-Process -FilePath $targetExe -WorkingDirectory $buildsDir -PassThru
        Write-Host "`n[HIVE CLI] SUCCESS: H.I.V.E. active! PID: $($proc.Id)" -ForegroundColor Green
        Write-Host "Binary: $targetExe" -ForegroundColor Cyan
        Write-Host "12 Autonomous Agents: Standby / Operational" -ForegroundColor DarkCyan
    } else {
        Write-Host "  [3/3] Launching in Unity Editor..." -ForegroundColor Green
        $logFile = Join-Path $ProjectRoot "launch.log"
        $unityArgs = "-projectPath `"$ProjectRoot`" -logFile `"$logFile`""
        $proc = Start-Process -FilePath $UnityExe -ArgumentList $unityArgs -PassThru
        Write-Host "`n[HIVE CLI] SUCCESS: H.I.V.E. launched in Editor! PID: $($proc.Id)" -ForegroundColor Green
    }
}

function Invoke-Gog {
    param([string[]]$ArgsList)
    $userProf = [Environment]::GetFolderPath("UserProfile")
    $gogBat = Join-Path $userProf "gog-wrapper\gog.bat"
    $gogExe = Join-Path $userProf "go\bin\gog.exe"

    if (Test-Path $gogBat) {
        & "$gogBat" @ArgsList
    } elseif (Test-Path $gogExe) {
        & "$gogExe" @ArgsList
    } else {
        Write-Host "[HIVE CLI] Error: gog CLI not found at $gogBat or $gogExe" -ForegroundColor Red
    }
}

# Main Dispatch
switch ($Command) {
    "launch"  { Show-Header; Invoke-Launch }
    "run"     { Show-Header; Invoke-Launch }
    "build"   { Show-Header; Invoke-Build }
    "capture" { Show-Header; Invoke-Capture -ArgsList $RemainingArgs }
    "test"    { Show-Header; Invoke-Test }
    "scenes"  { Show-Header; Invoke-Scenes }
    "doctor"  { Show-Header; Invoke-Doctor }
    "gog"     { Invoke-Gog -ArgsList $RemainingArgs }
    "google"  { Invoke-Gog -ArgsList $RemainingArgs }
    default   { Show-Help }
}
