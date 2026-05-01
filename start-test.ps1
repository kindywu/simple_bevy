# Start platform, server, and two clients for testing
$ErrorActionPreference = "Stop"

Write-Host "=== Starting platform ==="
$platform = Start-Process cargo -ArgumentList "run","-p","platform" -PassThru

Write-Host "Waiting for platform to start..."
Start-Sleep -Seconds 3

Write-Host "=== Starting server ==="
$server = Start-Process cargo -ArgumentList "run","-p","server" -PassThru

Write-Host "Waiting for server to start..."
Start-Sleep -Seconds 2

Write-Host "=== Starting client 1 ==="
$client1 = Start-Process cargo -ArgumentList "run","-p","client" -PassThru

Start-Sleep -Seconds 1

Write-Host "=== Starting client 2 ==="
$client2 = Start-Process cargo -ArgumentList "run","-p","client" -PassThru

Write-Host ""
Write-Host "All processes started:"
Write-Host "  Platform PID: $($platform.Id)"
Write-Host "  Server PID:   $($server.Id)"
Write-Host "  Client 1 PID: $($client1.Id)"
Write-Host "  Client 2 PID: $($client2.Id)"
Write-Host ""
Write-Host "Press any key to stop all processes..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

$platform.Kill()
$server.Kill()
$client1.Kill()
$client2.Kill()
Write-Host "All processes stopped"
