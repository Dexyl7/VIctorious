# Like sweep.ps1, but each pass starts where the last one crashed: the
# pipelines that built are noted in sweep_done.txt and left out next time.
param([int]$Rounds = 200)
Set-Location C:\VIctory3
$exe = "C:\VIctory3\target\diag\victory.exe"
$rpf = "D:\Me Pirate\GTAVI\shaders.rpf"
$done = "C:\VIctory3\sweep_done.txt"
foreach ($k in @("VICTORY_ABLATE", "VICTORY_ABLATE_RANGE", "VICTORY_DXIL_FLAGS_OR", "VICTORY_NO_WAVE_SIZE", "VICTORY_SPLIT_FMAD", "VICTORY_SHADER_OPT", "VICTORY_TRANSLATE_ONLY")) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
if (-not (Test-Path $done)) { Set-Content $done "" -Encoding ascii -NoNewline }
$env:VICTORY_TRANSLATE_CS_ONLY = "1"
$env:VICTORY_TRANSLATE_TRACE = "1"
$env:VICTORY_TRANSLATE_SKIP_FILE = $done
for ($i = 1; $i -le $Rounds; $i++) {
    $skip = "$(Get-Content C:\VIctory3\crashers_list.txt -Raw)".Trim()
    $env:VICTORY_TRANSLATE_SKIP = $skip
    $n = ($skip -split ",").Count
    $ok = @(Get-Content $done | Where-Object { $_.Trim() }).Count
    Write-Host "Pass $i : $ok built so far, $n crashers listed..."
    & $exe translate --pipelines --jobs 1 $rpf *> C:\VIctory3\sweep_out.txt
    $text = (Get-Content C:\VIctory3\sweep_out.txt -Raw) -replace "\r?\n", ""
    $found = [regex]::Matches($text, "building pipeline .*?#([0-9a-f]{16})")
    $hashes = @($found | ForEach-Object { $_.Groups[1].Value })
    if ($LASTEXITCODE -ne -1073741819) {
        Write-Host "Pass $i built the rest ($($hashes.Count)) without a driver crash. Done: $n crashers listed."
        break
    }
    if ($hashes.Count -eq 0) { Write-Host "Crashed but no shader named. Stopping."; break }
    $bad = $hashes[$hashes.Count - 1]
    if ($skip -split "," -contains $bad) { Write-Host "Crashed on $bad, already listed. Stopping."; break }
    Write-Host "  crashes the driver: $bad ($($hashes.Count - 1) built this pass)"
    if ($hashes.Count -gt 1) { Add-Content $done ($hashes[0..($hashes.Count - 2)] -join "`r`n") -Encoding ascii }
    "$skip,$bad" | Set-Content C:\VIctory3\crashers_list.txt -NoNewline
}
