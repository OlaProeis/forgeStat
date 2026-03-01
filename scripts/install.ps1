#!/usr/bin/env pwsh
# forgeStat Windows Installer
# This script downloads and installs forgeStat on Windows systems

$ErrorActionPreference = "Stop"

# Configuration
$Repo = "olaproeis/forgeStat"
$BinaryName = "forgeStat"
$InstallDir = "$env:LOCALAPPDATA\forgeStat"
$Version = "latest"

# Colors for output
$Green = "`e[32m"
$Red = "`e[31m"
$Yellow = "`e[33m"
$Blue = "`e[34m"
$Reset = "`e[0m"

function Write-Info($Message) {
    Write-Host "${Blue}[INFO]${Reset} $Message"
}

function Write-Success($Message) {
    Write-Host "${Green}[SUCCESS]${Reset} $Message"
}

function Write-Warning($Message) {
    Write-Host "${Yellow}[WARNING]${Reset} $Message"
}

function Write-Error($Message) {
    Write-Host "${Red}[ERROR]${Reset} $Message"
}

function Get-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { return "x86_64-pc-windows-msvc" }
        "ARM64" { return "aarch64-pc-windows-msvc" }
        default {
            Write-Error "Unsupported architecture: $arch"
            exit 1
        }
    }
}

function Get-LatestVersion {
    Write-Info "Fetching latest version..."
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -ErrorAction Stop
        return $release.tag_name
    }
    catch {
        Write-Error "Failed to fetch latest version. Please check your internet connection and try again."
        exit 1
    }
}

function Download-ForgeStat($Version, $Architecture) {
    $url = "https://github.com/$Repo/releases/download/$Version/forgeStat-$Version-$Architecture.zip"
    $tempDir = [System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString()
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    
    $zipFile = Join-Path $tempDir "forgeStat.zip"
    
    Write-Info "Downloading forgeStat $Version for $Architecture..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $zipFile -ErrorAction Stop
        Write-Success "Download complete"
        return $zipFile, $tempDir
    }
    catch {
        Write-Error "Failed to download forgeStat. Please check the version and architecture."
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
        exit 1
    }
}

function Install-ForgeStat($ZipFile, $TempDir) {
    Write-Info "Extracting archive..."
    Expand-Archive -Path $ZipFile -DestinationPath $TempDir -Force
    
    # Find the binary (might be in a subdirectory)
    $binary = Get-ChildItem -Path $TempDir -Recurse -Filter "$BinaryName.exe" | Select-Object -First 1
    if (-not $binary) {
        Write-Error "Could not find $BinaryName.exe in the downloaded archive"
        Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
        exit 1
    }
    
    Write-Info "Installing to $InstallDir..."
    
    # Create install directory
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    
    # Copy binary
    Copy-Item -Path $binary.FullName -Destination $InstallDir -Force
    
    # Cleanup temp directory
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    
    Write-Success "Binary installed to $InstallDir\$BinaryName.exe"
}

function Add-ToPath {
    Write-Info "Adding forgeStat to PATH..."
    
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($currentPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$InstallDir", "User")
        Write-Success "Added $InstallDir to PATH"
        Write-Warning "Please restart your terminal or run 'refreshenv' to use the 'forgeStat' command immediately"
    }
    else {
        Write-Info "forgeStat is already in PATH"
    }
}

function Create-StartMenuShortcut {
    Write-Info "Creating Start Menu shortcut..."
    
    $startMenuPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\forgeStat"
    New-Item -ItemType Directory -Path $startMenuPath -Force | Out-Null
    
    $WshShell = New-Object -comObject WScript.Shell
    $Shortcut = $WshShell.CreateShortcut("$startMenuPath\forgeStat.lnk")
    $Shortcut.TargetPath = "$InstallDir\$BinaryName.exe"
    $Shortcut.WorkingDirectory = "$InstallDir"
    $Shortcut.Description = "forgeStat - GitHub Repository Dashboard"
    $Shortcut.Save()
    
    Write-Success "Start Menu shortcut created"
}

function Test-Installation {
    Write-Info "Testing installation..."
    
    $binaryPath = "$InstallDir\$BinaryName.exe"
    if (Test-Path $binaryPath) {
        $version = & $binaryPath --version 2>&1
        Write-Success "forgeStat is installed and working: $version"
        return $true
    }
    else {
        Write-Error "Installation failed - binary not found at $binaryPath"
        return $false
    }
}

function Main {
    Write-Host @"
╔══════════════════════════════════════════════════════════════╗
║                   forgeStat Installer                        ║
║          A real-time GitHub repository dashboard             ║
╚══════════════════════════════════════════════════════════════╝
"@
    
    # Check if running as administrator (not required, but warn)
    $currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if ($currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Write-Warning "Running as administrator. Installation will be for the current user only."
    }
    
    # Get architecture
    $architecture = Get-Architecture
    Write-Info "Detected architecture: $architecture"
    
    # Get version (latest or specified)
    if ($Version -eq "latest") {
        $Version = Get-LatestVersion
    }
    Write-Info "Installing version: $Version"
    
    # Download
    $zipFile, $tempDir = Download-ForgeStat -Version $Version -Architecture $architecture
    
    # Install
    Install-ForgeStat -ZipFile $zipFile -TempDir $tempDir
    
    # Add to PATH
    Add-ToPath
    
    # Create shortcut
    Create-StartMenuShortcut
    
    # Test
    if (Test-Installation) {
        Write-Host ""
        Write-Success "forgeStat has been successfully installed!"
        Write-Host ""
        Write-Info "Usage examples:"
        Write-Host "  forgeStat owner/repo              # Launch TUI"
        Write-Host "  forgeStat owner/repo --summary    # Quick summary"
        Write-Host "  forgeStat owner/repo --json       # Export JSON"
        Write-Host "  forgeStat --help                  # Show all options"
        Write-Host ""
        Write-Info "Documentation: https://github.com/$Repo"
    }
    else {
        Write-Error "Installation failed. Please try again or install manually."
        exit 1
    }
}

# Run main function
Main
