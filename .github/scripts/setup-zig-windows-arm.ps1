$ErrorActionPreference = 'Stop'

# The ARM-native Zig 0.16.0 compiler crashes while building Ghostty.
# Run the x64 compiler under Windows emulation; Cargo still selects the ARM target.
$archive = Join-Path $env:RUNNER_TEMP 'zig-x86_64-windows-0.16.0.zip'
$destination = Join-Path $env:RUNNER_TEMP 'zig-x64'
Invoke-WebRequest 'https://ziglang.org/download/0.16.0/zig-x86_64-windows-0.16.0.zip' -OutFile $archive
$expected = '68659eb5f1e4eb1437a722f1dd889c5a322c9954607f5edcf337bc3684a75a7e'
if ((Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
  throw 'Zig archive checksum mismatch'
}
Expand-Archive $archive -DestinationPath $destination -Force
$zigDirectory = Join-Path $destination 'zig-x86_64-windows-0.16.0'
$zigDirectory | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
& (Join-Path $zigDirectory 'zig.exe') version
if ($LASTEXITCODE -ne 0) {
  throw 'Zig failed to start'
}
