# Lists every RaytracingResolve compute shader's hash into rt_resolve.txt
# (translation only, no GPU), and makes crashers_list.txt skip them: they
# cost GPU time but draw nothing while the fork's ray tracing is off.
Set-Location C:\VIctory3
foreach ($k in @("VICTORY_TRANSLATE_SKIP", "VICTORY_TRANSLATE_SKIP_FILE", "VICTORY_TRANSLATE_CS_ONLY", "VICTORY_TRANSLATE_TRACE", "VICTORY_TRANSLATE_ONLY", "VICTORY_EMIT_DUMP", "VICTORY_ABLATE", "VICTORY_ABLATE_RANGE")) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
.\target\diag\victory.exe translate --list "D:\Me Pirate\GTAVI\shaders.rpf" *> C:\VIctory3\list_out.txt
$text = (Get-Content C:\VIctory3\list_out.txt -Raw) -replace "\r?\n", ""
$hashes = [regex]::Matches($text, "RaytracingResolve[^#]{0,200}#([0-9a-f]{16})") | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
$hashes -join "," | Set-Content C:\VIctory3\rt_resolve.txt -NoNewline
Copy-Item C:\VIctory3\rt_resolve.txt C:\VIctory3\crashers_list.txt
Write-Host "$($hashes.Count) ray-tracing resolve shaders listed; play.ps1 will skip them."
