# Writes the DXIL and disassembly of every listed crasher and of a sample of
# compute shaders that built, for comparing them. Translation only: no GPU
# pipelines are built, so it can run beside the sweep.
Set-Location C:\VIctory3
$dxc = (Get-Command dxc.exe -ErrorAction SilentlyContinue).Source
if (-not $dxc) {
    $dxc = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Recurse -Filter dxc.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "x64" } | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
}
$bad = (Get-Content C:\VIctory3\crashers_list.txt -Raw).Trim() -split ","
$good = @(Get-Content C:\VIctory3\sweep_done.txt | Where-Object { $_.Trim() } | Get-Random -Count 200 -SetSeed 7)
foreach ($set in @(@("bad", $bad), @("good", $good))) {
    $dir = "C:\VIctory3\cmp_$($set[0])"
    New-Item -ItemType Directory -Force $dir | Out-Null
    foreach ($k in @("VICTORY_ABLATE", "VICTORY_ABLATE_RANGE", "VICTORY_TRANSLATE_SKIP", "VICTORY_TRANSLATE_SKIP_FILE", "VICTORY_TRANSLATE_CS_ONLY", "VICTORY_TRANSLATE_TRACE")) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
    $env:VICTORY_TRANSLATE_ONLY = ($set[1] -join ",")
    $env:VICTORY_EMIT_DUMP = $dir
    .\target\diag\victory.exe translate --list "D:\Me Pirate\GTAVI\shaders.rpf" *> $null
    $files = Get-ChildItem $dir -Filter *.dxil
    Write-Host "$($set[0]): $($files.Count) shaders written; disassembling..."
    foreach ($f in $files) {
        if (-not (Test-Path "$($f.FullName).txt")) { & $dxc -dumpbin $f.FullName | Out-File "$($f.FullName).txt" -Encoding utf8 }
    }
}
Write-Host "Done."
