$ErrorActionPreference = "Stop"

$owner = "dreygur"
$repo = "mcp-connect"
$binary = "mcp-connect"
$os = "windows"
$arch = "x86_64"

Write-Host "Fetching latest release info..."
$release = Invoke-RestMethod "https://api.github.com/repos/$owner/$repo/releases/latest"
$tag = $release.tag_name
if (-not $tag) {
    Write-Error "❌ Could not determine latest release tag."
    exit 1
}

$file = "$binary-$os-$arch.zip"
$checksumFile = "$file.sha256"
$baseUrl = "https://github.com/$owner/$repo/releases/download/$tag"

$tmpFile = Join-Path $env:TEMP $file
$tmpChecksum = Join-Path $env:TEMP $checksumFile
$target = "C:\Program Files\$binary"

Write-Host "📦 Downloading $file (version $tag)..."
Invoke-WebRequest "$baseUrl/$file" -OutFile $tmpFile
Invoke-WebRequest "$baseUrl/$checksumFile" -OutFile $tmpChecksum

Write-Host "🔐 Verifying checksum..."
$expectedHash = (Get-Content $tmpChecksum).Split(" ")[0].Trim()
$actualHash = (Get-FileHash $tmpFile -Algorithm SHA256).Hash.ToLower()
if ($expectedHash -ne $actualHash) {
    Write-Error "❌ Checksum verification failed! Expected $expectedHash but got $actualHash."
    exit 1
}

Write-Host "📂 Extracting to $target..."
if (Test-Path $target) { Remove-Item -Recurse -Force $target }
Expand-Archive -Path $tmpFile -DestinationPath $target -Force

$exePath = Join-Path $target "$binary.exe"
if (-not (Test-Path $exePath)) {
    Write-Error "❌ Executable not found after extraction!"
    exit 1
}

Write-Host "⚙️ Adding $target to PATH..."
$env:Path += ";$target"
[Environment]::SetEnvironmentVariable("Path", $env:Path, [EnvironmentVariableTarget]::Machine)

Write-Host "🧹 Cleaning up temp files..."
Remove-Item -Path $tmpFile -Force -ErrorAction SilentlyContinue
Remove-Item -Path $tmpChecksum -Force -ErrorAction SilentlyContinue

Write-Host "✅ Installed $binary successfully!"
& $exePath --version
