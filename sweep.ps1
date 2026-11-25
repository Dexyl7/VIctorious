# Builds every compute pipeline of the game's shaders offline (no game run,
# no network needed), one at a time, and adds each one that crashes the AMD
# driver compiler to crashers_list.txt, until a pass builds them all.
param([int]$Rounds = 60)
Set-Location C:\VIctory3
$exe = "C:\VIctory3\target\diag\victory.exe"
$rpf = "D:\Me Pirate\GTAVI\shaders.rpf"
foreach ($k in @("VICTORY_ABLATE", "VICTORY_ABLATE_RANGE", "VICTORY_DXIL_FLAGS_OR", "VICTORY_NO_WAVE_SIZE", "VICTORY_SPLIT_FMAD", "VICTORY_SHADER_OPT", "VICTORY_TRANSLATE_ONLY")) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$env:VICTORY_TRANSLATE_CS_ONLY = "1"
$env:VICTORY_TRANSLATE_TRACE = "1"
for ($i = 1; $i -le $Rounds; $i++) {
    $skip = (Get-Content C:\VIctory3\crashers_list.txt -Raw).Trim()
    $env:VICTORY_TRANSLATE_SKIP = $skip
    $n = ($skip -split ",").Count
    Write-Host "Pass $i (skipping $n)..."
    & $exe translate --pipelines --jobs 1 $rpf *> C:\VIctory3\sweep_out.txt
    if ($LASTEXITCODE -ne -1073741819) {
        Write-Host "Pass $i built every compute pipeline without a driver crash (exit $LASTEXITCODE). Done: $n on the list."
        break
    }
    # (PowerShell wraps the captured lines: join them before looking.)
    $text = (Get-Content C:\VIctory3\sweep_out.txt -Raw) -replace "\r?\n", ""
    $found = [regex]::Matches($text, "building pipeline .*?#([0-9a-f]{16})")
    if ($found.Count -eq 0) { Write-Host "Crashed but no shader named. Stopping."; break }
    $hash = $found[$found.Count - 1].Groups[1].Value
    if ($skip -split "," -contains $hash) { Write-Host "Crashed on $hash, already listed. Stopping."; break }
    Write-Host "  crashes the driver: $hash"
    "$skip,$hash" | Set-Content C:\VIctory3\crashers_list.txt -NoNewline
}
