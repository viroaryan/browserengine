<#
.SYNOPSIS
Builds libnirvana_engine.so for Android ABIs using cargo-ndk.

.DESCRIPTION
Compiles the nirvana_android crate into native shared libraries and installs
them into the android/app/src/main/jniLibs directory.
#>

param(
    [string]$TargetAbi = "all",
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = Split-Path -Parent $scriptDir
$jniLibsDir = Join-Path $rootDir "android\app\src\main\jniLibs"

Write-Host "==> Checking cargo-ndk installation..." -ForegroundColor Cyan
if (-not (Get-Command "cargo-ndk" -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-ndk..." -ForegroundColor Yellow
    cargo install cargo-ndk
}

$targets = @("arm64-v8a", "x86_64")
if ($TargetAbi -ne "all") {
    $targets = @($TargetAbi)
}

foreach ($abi in $targets) {
    Write-Host "`n==> Building Nirvana Engine for ABI: $abi ($Profile)..." -ForegroundColor Green
    $extraArgs = @()
    if ($Profile -eq "release") {
        $extraArgs += "--release"
    }

    cargo ndk -t $abi -o $jniLibsDir build -p nirvana_android @extraArgs
}

Write-Host "`n==> Build complete! Libraries output to: $jniLibsDir" -ForegroundColor Green
