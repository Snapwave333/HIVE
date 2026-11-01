param(
    [Parameter(Position = 0)]
    [string]$Command = "help",
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Rest
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$TauriDir = Join-Path $ProjectRoot "src-tauri"
$DebugExe = Join-Path $TauriDir "target\debug\hive-desktop.exe"
$ReleaseExe = Join-Path $TauriDir "target\release\hive-desktop.exe"
Set-Location $ProjectRoot

function Show-Header {
    Write-Host ""
    Write-Host "  H.I.V.E. Desktop CLI" -ForegroundColor Green
    Write-Host "  ---------------------" -ForegroundColor DarkGreen
}

function Assert-Tool([string]$Name, [string]$Hint) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Host "  [FAIL] '$Name' not found. $Hint" -ForegroundColor Red
        exit 1
    }
}

function Invoke-Npm([string[]]$NpmArgs) {
    & npm @NpmArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

function Ensure-Deps {
    if (-not (Test-Path (Join-Path $ProjectRoot "node_modules"))) {
        Write-Host "  Installing npm dependencies..." -ForegroundColor Cyan
        Invoke-Npm @("ci", "--no-audit", "--no-fund")
    }
}

function Invoke-Dev {
    Assert-Tool "npm" "Install Node.js 20+."
    Assert-Tool "cargo" "Install Rust from https://rustup.rs."
    Ensure-Deps
    Invoke-Npm @("run", "build")
    & cargo run --manifest-path (Join-Path $TauriDir "Cargo.toml")
    exit $LASTEXITCODE
}

function Invoke-Build {
    Assert-Tool "npm" "Install Node.js 20+."
    Assert-Tool "cargo" "Install Rust from https://rustup.rs."
    Ensure-Deps
    Invoke-Npm @("run", "tauri", "build")
    Write-Host "  [OK] Installers are in src-tauri\target\release\bundle" -ForegroundColor Green
}

function Invoke-Test {
    Assert-Tool "npm" "Install Node.js 20+."
    Assert-Tool "cargo" "Install Rust from https://rustup.rs."
    Ensure-Deps
    Write-Host "  [1/2] Constitution check, typecheck and frontend build..." -ForegroundColor Cyan
    Invoke-Npm @("test")
    Write-Host "  [2/2] Rust tests..." -ForegroundColor Cyan
    & cargo test --manifest-path (Join-Path $TauriDir "Cargo.toml")
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host "  [OK] All checks passed." -ForegroundColor Green
}

function Invoke-Launch {
    $exe = if (Test-Path $ReleaseExe) { $ReleaseExe } elseif (Test-Path $DebugExe) { $DebugExe } else { $null }
    if (-not $exe) {
        Write-Host "  No build found. Building a debug binary first..." -ForegroundColor Yellow
        Ensure-Deps
        Invoke-Npm @("run", "build")
        & cargo build --manifest-path (Join-Path $TauriDir "Cargo.toml")
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        $exe = $DebugExe
    }
    Start-Process -FilePath $exe -WindowStyle Maximized
    Write-Host "  [OK] H.I.V.E. launched from $exe (tray icon active, Ctrl+Shift+H toggles the window)" -ForegroundColor Green
}

function Invoke-Doctor {
    Write-Host "  Checking environment..." -ForegroundColor Cyan

    foreach ($tool in @(
        @{ Name = "node"; Hint = "Install Node.js 20+." },
        @{ Name = "npm"; Hint = "Install Node.js 20+." },
        @{ Name = "cargo"; Hint = "Install Rust from https://rustup.rs." },
        @{ Name = "curl.exe"; Hint = "Agents call NVIDIA NIM through curl.exe (ships with Windows 10+)." }
    )) {
        $cmd = Get-Command $tool.Name -ErrorAction SilentlyContinue
        if ($cmd) {
            Write-Host "  [OK] $($tool.Name): $($cmd.Source)" -ForegroundColor Green
        } else {
            Write-Host "  [FAIL] $($tool.Name) not found. $($tool.Hint)" -ForegroundColor Red
        }
    }

    $agentsDir = Join-Path $ProjectRoot "agents"
    $agents = Get-ChildItem -Path $agentsDir -Filter "*.json" -ErrorAction SilentlyContinue
    if ($agents.Count -eq 12) {
        $bad = @()
        foreach ($a in $agents) {
            try { Get-Content $a.FullName -Raw | ConvertFrom-Json | Out-Null } catch { $bad += $a.Name }
        }
        if ($bad.Count -eq 0) {
            Write-Host "  [OK] 12 agent definitions parse cleanly in agents/." -ForegroundColor Green
        } else {
            Write-Host "  [FAIL] Invalid JSON in: $($bad -join ', ')" -ForegroundColor Red
        }
    } else {
        Write-Host "  [FAIL] Expected 12 agent definitions in agents/, found $($agents.Count)." -ForegroundColor Red
    }

    $nimKey = $null
    $nimScope = $null
    foreach ($scope in @("Process", "User", "Machine")) {
        foreach ($name in @("NVIDIA_API_KEY", "NIM_API_KEY")) {
            if (-not $nimKey) {
                $val = [Environment]::GetEnvironmentVariable($name, $scope)
                if ($val) { $nimKey = $val; $nimScope = "$name ($scope)" }
            }
        }
    }
    if ($nimKey) {
        Write-Host "  [OK] NIM API key detected: $nimScope." -ForegroundColor Green
    } else {
        Write-Host "  [WARN] NVIDIA_API_KEY not set. Agents can't reach their models and will say so in chat until it's set." -ForegroundColor Yellow
    }

    $gog = Get-Command "gog" -ErrorAction SilentlyContinue
    $userProf = [Environment]::GetFolderPath("UserProfile")
    $gogPaths = @($env:GOG_CLI_PATH, (Join-Path $userProf "gog-wrapper\gog.bat"), (Join-Path $userProf "go\bin\gog.exe")) | Where-Object { $_ -and (Test-Path $_) }
    $gogPath = if ($gogPaths) { $gogPaths[0] } elseif ($gog) { $gog.Source } else { $null }
    if ($gogPath) {
        Write-Host "  [OK] Google CLI found: $gogPath" -ForegroundColor Green
        try {
            $auth = & $gogPath auth list --no-input 2>$null | Out-String
            if ($LASTEXITCODE -eq 0 -and $auth.Trim()) {
                Write-Host "  [OK] gog auth: $($auth.Trim().Split([Environment]::NewLine)[0])" -ForegroundColor Green
            } else {
                Write-Host "  [WARN] gog is installed but not signed in. Run 'gog auth login'." -ForegroundColor Yellow
            }
        } catch {
            Write-Host "  [WARN] gog auth check failed: $($_.Exception.Message)" -ForegroundColor Yellow
        }
    } else {
        Write-Host "  [WARN] Google CLI (gog) not found. Set GOG_CLI_PATH or add it to PATH to enable /gog." -ForegroundColor Yellow
    }

    $vault = if ($env:HIVE_OBSIDIAN_VAULT) { $env:HIVE_OBSIDIAN_VAULT } else { Join-Path ([Environment]::GetFolderPath("MyDocuments")) "HIVE-Vault" }
    if (Test-Path $vault) {
        Write-Host "  [OK] Obsidian vault: $vault" -ForegroundColor Green
    } else {
        Write-Host "  [INFO] Obsidian vault will be created on first use at $vault" -ForegroundColor DarkCyan
    }

    $dataDir = Join-Path $env:APPDATA "Lushapps\HIVE\data\hive.db"
    if (Test-Path $dataDir) {
        Write-Host "  [OK] Database: $dataDir" -ForegroundColor Green
    } else {
        Write-Host "  [INFO] No database yet. It's created on first launch at $dataDir" -ForegroundColor DarkCyan
    }
}

function Show-Help {
    Write-Host @"

  Usage: hive <command>

  dev       Build the frontend and run the desktop app from source
  build     Produce release installers with 'tauri build'
  test      Constitution check, typecheck, frontend build and Rust tests
  launch    Start the built app (builds a debug binary if none exists)
  doctor    Check tools, agent definitions, API key, Google CLI and vault
  help      Show this message

"@
}

Show-Header
switch ($Command.ToLower()) {
    "dev" { Invoke-Dev }
    "build" { Invoke-Build }
    "test" { Invoke-Test }
    "launch" { Invoke-Launch }
    "doctor" { Invoke-Doctor }
    default { Show-Help }
}
