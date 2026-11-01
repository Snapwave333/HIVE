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
}

function Show-Help {
    Show-Header
    Write-Host @"
USAGE:
  hive <command> [options]

COMMANDS:
  capture          Take headless 3D screenshots of operational scenes
  test             Execute comprehensive end-to-end integration test suite
  scenes           Rebuild and wire all 7 HQ scenes and cybernetic prefabs
  doctor           Inspect health of Unity, schemas, scenes, and API keys
  help             Show this help reference

CAPTURE OPTIONS:
  --all            Capture all 7 operational scenes (default: all)
  --scene <name>   Capture a specific scene (CommandChamber, Workspaces, etc.)
  --res <WxH>      Render resolution (e.g. 1920x1080, 3840x2160, 1280x720)
  --out <dir>      Output directory for screenshots (default: Screenshots)
  --mode <mode>    Camera mode: FirstPerson, ThirdPerson, Isometric, Overview

EXAMPLES:
  .\hive.cmd capture
  .\hive.cmd capture --scene CommandChamber --res 3840x2160 --out stills
  .\hive.cmd capture --scene Workspaces --mode Overview
  .\hive.cmd test
  .\hive.cmd doctor
"@
}

# Main Dispatch
switch ($Command) {
    "capture" { Show-Header; Invoke-Capture -ArgsList $RemainingArgs }
    "test"    { Show-Header; Invoke-Test }
    "scenes"  { Show-Header; Invoke-Scenes }
    "doctor"  { Show-Header; Invoke-Doctor }
    default   { Show-Help }
}
