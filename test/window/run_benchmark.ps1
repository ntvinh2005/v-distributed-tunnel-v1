$ScriptDir   = $PSScriptRoot
$ProjectRoot = Resolve-Path (Join-Path $ScriptDir "..\..")

Write-Host "ScriptDir: $ScriptDir"
Write-Host "ProjectRoot: $ProjectRoot"

$ServerExe      = Join-Path $ProjectRoot "target\release\server.exe"
$ClientExe      = Join-Path $ProjectRoot "target\release\client.exe"
$EchoExe        = Join-Path $ProjectRoot "test\simple-go-service-client-test\echo-server.exe"
$TunnelAdminExe = Join-Path $ProjectRoot "target\release\tunnel_admin.exe"
$ResultsDir     = Join-Path $ProjectRoot "results"
$ResultsFile    = Join-Path $ResultsDir ("bench-" + (Get-Date -Format 'yyyyMMdd-HHmm') + ".csv")
$ClientOutputFile = Join-Path $ProjectRoot "client_output.txt"

if (!(Test-Path $ResultsDir)) { New-Item -ItemType Directory -Path $ResultsDir }

Get-Process server,client,"echo-server","iperf3" -ErrorAction SilentlyContinue | Stop-Process -Force

Write-Host "Starting tunnel server..."
$ServerProc = Start-Process -FilePath $ServerExe -WorkingDirectory $ProjectRoot -PassThru
Start-Sleep -Seconds 4

$NodeId = "node$([System.Guid]::NewGuid().ToString('N').Substring(0, 6))"
$NodePassword = ""

$tunnelAdminOutput = & $TunnelAdminExe add $NodeId
Write-Host "`nTunnel admin output:`n$tunnelAdminOutput`n"

foreach ($line in $tunnelAdminOutput -split "`n") {
    if ($line -match "Password:\s*([^\s]+)") {
        $NodePassword = $matches[1]
    }
}

Write-Host "Node ID: $NodeId"
Write-Host "Node Password: $NodePassword"

if (-not $NodePassword) {
    Write-Host "FATAL: No node password found in tunnel_admin output. Exiting."
    exit 1
}

Write-Host "Starting tunnel client (with node ID/password)..."
if (Test-Path $ClientOutputFile) { Remove-Item $ClientOutputFile }
$ClientArgs = @("--node-id", $NodeId, "--password", $NodePassword)
$ClientProc = Start-Process -FilePath $ClientExe -ArgumentList $ClientArgs -WorkingDirectory $ProjectRoot -RedirectStandardOutput $ClientOutputFile -WindowStyle Hidden -PassThru

#here we extract the assigned port, pattern match in the line Assigned port: <port>
$AssignedPort = ""
for ($i=0; $i -lt 40; $i++) {
    if (Test-Path $ClientOutputFile) {
        foreach ($line in Get-Content $ClientOutputFile) {
            if ($line -match "Assigned port:\s*(\d+)") {
                $AssignedPort = $matches[1]
                break
            }
        }
        if ($AssignedPort) { break }
    }
    Start-Sleep -Milliseconds 250
}
if (-not $AssignedPort) {
    Write-Host "FATAL: Could not find assigned port in client output."
    $ClientProc | Stop-Process -Force
    $ServerProc | Stop-Process -Force
    exit 1
}
Write-Host "Assigned Tunnel Port: $AssignedPort"

Write-Host "Starting Go echo server..."
$EchoProc = Start-Process -FilePath $EchoExe -ArgumentList ":$AssignedPort" -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 1

Write-Host ""

"Streams,Throughput_Mbps,Transfer_MBytes" | Out-File $ResultsFile

$streamsList = @(1, 5, 10, 20, 50)
foreach ($streams in $streamsList) {
    Write-Host "Running iperf3 with $streams parallel streams..."
    $IperfResult = & $IperfExe -c 127.0.0.1 -p $AssignedPort -P $streams

    $ThroughputLine = $IperfResult | Select-String "SUM.*Mbits/sec" | Select-Object -Last 1
    $ThroughputMbps = ""
    $TransferMB = ""
    if ($ThroughputLine) {
        $parts = $ThroughputLine -split "\s+"
        $ThroughputMbps = $parts[-2]
        $TransferMB = $parts[-5]
    }
    else {
        $ThroughputMbps = "ERROR"
        $TransferMB = "ERROR"
    }
    "$streams,$ThroughputMbps,$TransferMB" | Add-Content $ResultsFile
    Start-Sleep -Seconds 2
}

Write-Host "Stopping all servers..."
$EchoProc     | Stop-Process -Force
$ServerProc   | Stop-Process -Force
$IperfSrvProc | Stop-Process -Force
$ClientProc   | Stop-Process -Force

Write-Host "Cleaning up node from tunnel_admin..."
$tunnelAdminDelete = & $TunnelAdminExe delete $NodeId
Write-Host $tunnelAdminDelete

Write-Host "Benchmark complete! Results in $ResultsFile"
Get-Content $ResultsFile
