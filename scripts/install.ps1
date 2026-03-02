$ErrorActionPreference = "Stop"

$Repo       = "OlaProeis/forgeStat"
$Binary     = "forgeStat"
$InstallDir = "$env:LOCALAPPDATA\$Binary"

function Write-Info($m)  { Write-Host "[INFO] $m" -ForegroundColor Cyan }
function Write-Ok($m)    { Write-Host "[OK]   $m" -ForegroundColor Green }
function Write-Err($m)   { Write-Host "[ERR]  $m" -ForegroundColor Red; exit 1 }

function Refresh-Path {
  $machine = [Environment]::GetEnvironmentVariable("PATH", "Machine")
  $user    = [Environment]::GetEnvironmentVariable("PATH", "User")
  $env:PATH = "$machine;$user"
}

$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
  "AMD64" { "x86_64-pc-windows-msvc" }
  "ARM64" { "aarch64-pc-windows-msvc" }
  default { Write-Err "Unsupported arch: $env:PROCESSOR_ARCHITECTURE" }
}

Write-Info "forgeStat Windows Installer"
Write-Info "Architecture: $arch"

try {
  $release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
  $version = $release.tag_name
} catch { Write-Err "Cannot fetch latest release: $_" }
Write-Info "Version: $version"

$tmp = Join-Path $env:TEMP ([Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmp -Force | Out-Null

# --- Try MSI first ---
$msiUrl  = "https://github.com/$Repo/releases/download/$version/$Binary-$version-$arch.msi"
$msiPath = Join-Path $tmp "$Binary.msi"
try {
  Write-Info "Downloading MSI..."
  Invoke-WebRequest -Uri $msiUrl -OutFile $msiPath -UseBasicParsing -ErrorAction Stop
  Write-Info "Running MSI installer (silent)..."
  $p = Start-Process msiexec.exe -ArgumentList "/i","`"$msiPath`"","/quiet","/norestart" -Wait -PassThru
  if ($p.ExitCode -eq 0) {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    Refresh-Path
    Write-Ok "Installed via MSI. Run: forgeStat --version"
    exit 0
  }
  Write-Info "MSI returned exit code $($p.ExitCode), falling back to ZIP..."
} catch {
  Write-Info "MSI not available, falling back to ZIP..."
}

# --- ZIP fallback ---
$zipUrl  = "https://github.com/$Repo/releases/download/$version/$Binary-$version-$arch.zip"
$zipPath = Join-Path $tmp "$Binary.zip"
try {
  Write-Info "Downloading ZIP..."
  Invoke-WebRequest -Uri $zipUrl -OutFile $zipPath -UseBasicParsing
  Expand-Archive -Path $zipPath -DestinationPath $tmp -Force

  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  Copy-Item (Join-Path $tmp "$Binary.exe") $InstallDir -Force

  $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
  if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$InstallDir", "User")
    Write-Info "Added $InstallDir to PATH."
  }
  Refresh-Path
  Write-Ok "Installed. Run: forgeStat --version"
} catch {
  Write-Err "Installation failed: $_"
} finally {
  Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
