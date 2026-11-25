# Builds each crashing shader's pipeline offline (no game) with the knobs
# given, and lists which still crash the AMD driver compiler.
# Example: .\trial.ps1 -Knobs @{ VICTORY_DXIL_FLAGS_OR = "1" } -Label skipopt
param([hashtable]$Knobs = @{}, [string]$Label = "baseline", [string]$Hashes = "")
Set-Location C:\VIctory3
foreach ($k in @("VICTORY_TRANSLATE_SKIP", "VICTORY_TRANSLATE_SKIP_FILE", "VICTORY_TRANSLATE_CS_ONLY", "VICTORY_TRANSLATE_TRACE", "VICTORY_EMIT_DUMP")) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$exe = "C:\VIctory3\target\diag\victory.exe"
$rpf = "D:\Me Pirate\GTAVI\shaders.rpf"
$list = if ($Hashes) { $Hashes -split "," } else { "$(Get-Content C:\VIctory3\crashers_list.txt -Raw)".Trim() -split "," }
foreach ($k in @("VICTORY_ABLATE", "VICTORY_ABLATE_RANGE", "VICTORY_DXIL_FLAGS_OR", "VICTORY_NO_WAVE_SIZE", "VICTORY_SPLIT_FMAD", "VICTORY_SHADER_OPT")) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
foreach ($k in $Knobs.Keys) { Set-Item "Env:$k" $Knobs[$k] }
$crash = 0; $ok = 0; $lines = @()
foreach ($h in $list) {
    $env:VICTORY_TRANSLATE_ONLY = $h.Trim()
    & $exe translate --pipelines $rpf *> C:\VIctory3\trial_out.txt
    $text = Get-Content C:\VIctory3\trial_out.txt -Raw
    if ($LASTEXITCODE -eq -1073741819) { $crash++; $r = "CRASH" }
    elseif ($text -match "pipeline: ") { $r = "refused (no crash, but not built)" }
    elseif ($text -match "\s+0 emitted of [1-9]\d* (compute|mesh|vertex)") { $r = "does not translate (nothing built)" }
    else { $ok++; $r = "built" }
    $lines += "$h $r"
    Write-Host "$h $r"
}
$sum = "[$Label] $ok built, $crash crashed of $($list.Count)"
Write-Host $sum
($sum, $lines) | Out-File "C:\VIctory3\trial_$Label.txt" -Encoding utf8
