# Finds shaders whose pipelines crash the AMD driver compiler (amdxc64.dll):
# runs the game with one pipeline worker, reads the pipeline being made when
# it crashed, adds it to the skip list, and repeats. Turn the network off first.
param(
    [string]$Skip = "1c02f8131c508c75,27ce027e7188bed3,842de0c32bc8dcc5,61d7c3333b73892d,077869cd9b1456ee,a4df980a1c65fdbe,7b956e1f6d56609f,d0130073bb6f7a7d,072f5fe96efd27e1,3a6594961b9247e7,8a13d3da7f64b8d1,2e28ea6fb9dad3c2,d2c52fcd936985b4,231d2daa66789312,3fa1fda35402c93f,501f0865fdf45dfd,1e4947eada3d9ee3,7fb216a0c6dee3a9,dbba7e080e26ec20",
    [int]$Rounds = 25,
    [int]$Seconds = 240
)
Set-Location C:\VIctory3
$exe = "C:\VIctory3\target\diag\victory.exe"
$game = "D:\Me Pirate\GTAVI\game_scarlett_master.exe"
$env:VICTORY_D3D12_DRED = "1"
$env:VICTORY_PIPELINE_WORKERS = "1"
for ($i = 1; $i -le $Rounds; $i++) {
    $env:VICTORY_SKIP_PIPELINES = $Skip
    $out = "C:\VIctory3\crashers_$i.out.txt"
    $err = "C:\VIctory3\crashers_$i.err.txt"
    Write-Host "Round $i, skipping: $Skip"
    $p = Start-Process -FilePath $exe -ArgumentList "run", "`"$game`"", "--execute" `
        -RedirectStandardOutput $out -RedirectStandardError $err -PassThru -NoNewWindow
    if (-not $p.WaitForExit($Seconds * 1000)) {
        Write-Host "Still running after $Seconds s with no crash: stopping it. Done."
        Stop-Process -Id $p.Id -Force
        break
    }
    $log = (Get-Content $out, $err -ErrorAction SilentlyContinue)
    if (-not ($log | Select-String -SimpleMatch "amdxc64.dll")) {
        Write-Host "Exited without a driver compiler crash. Done (see $err)."
        break
    }
    $making = $log | Select-String -Pattern "making host pipeline.*?(?:compute|vertex|pixel|mesh|amplification) ([0-9a-f]{16})" | Select-Object -Last 1
    if (-not $making) { Write-Host "Driver crash, but no pipeline named. Stopping."; break }
    $hash = $making.Matches[0].Groups[1].Value
    if ($Skip -split "," -contains $hash) { Write-Host "Crashed on $hash, already skipped. Stopping."; break }
    Write-Host "  driver crashed making $hash"
    $Skip = "$Skip,$hash"
}
Write-Host "Final skip list: $Skip"
$Skip | Out-File C:\VIctory3\crashers_list.txt
