# Ensure the script runs as Administrator for proper installation

$rustInstallerUrl = "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
Write-Host "Installing Rust..."

$rustInstaller = "$env:TEMP\rustup-init.exe"

Invoke-WebRequest -Uri $rustInstallerUrl -OutFile $rustInstaller

Start-Process "$rustInstaller" -Wait
Remove-Item $rustInstaller
rustup update
Write-Host "Verifying Rust installation..."
rustc --version

$gstreamerUrl = "https://gstreamer.freedesktop.org/data/pkg/windows/1.25.1/msvc/gstreamer-1.0-devel-msvc-x86_64-1.25.1.msi"
$gstreamerInstaller = "$env:TEMP\gstreamer-1.0-devel-msvc-x86_64-1.25.1.msi"

Write-Host "Downloading GStreamer Development files..."
Invoke-WebRequest -Uri $gstreamerUrl -OutFile $gstreamerInstaller

Write-Host "Installing GStreamer Development files"
Start-Process msiexec.exe -ArgumentList "/i", $gstreamerInstaller, "/qn", "/norestart", "ADDLOCAL=ALL" -Wait

Remove-Item $gstreamerInstaller


$gstreamerUrl = "https://gstreamer.freedesktop.org/data/pkg/windows/1.25.1/msvc/gstreamer-1.0-msvc-x86_64-1.25.1.msi"
$gstreamerInstaller = "$env:TEMP\gstreamer-1.0-msvc-x86_64-1.25.1.msi"

Write-Host "Downloading GStreamer Runtime files..."
Invoke-WebRequest -Uri $gstreamerUrl -OutFile $gstreamerInstaller

Write-Host "Installing GStreamer Runtime"
Start-Process msiexec.exe -ArgumentList "/i", $gstreamerInstaller, "/qn", "/norestart", "ADDLOCAL=ALL" -Wait

Remove-Item $gstreamerInstaller

# Add GStreamer to the user path
$newPath = "C:\Program Files\gstreamer\1.0\msvc_x86_64\bin"

if (-not (Test-Path $newPath)) {
    Write-Error "PATH doesn't exist"
    exit
}

$currentPath = [Environment]::GetEnvironmentVariable('Path', 'User')

if ($currentPath -split ';' -contains $newPath) {
    Write-Host "PATH already exists"
    exit
}

[Environment]::SetEnvironmentVariable('Path', "$currentPath;$newPath", 'User')