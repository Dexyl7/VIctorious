# Writes one shader's translated DXIL and its disassembly to C:\VIctory3\dump.
param([string]$Hash = "a1266cc7391ae0d6", [string]$Source = "D:\Me Pirate\GTAVI\shaders.rpf")
Set-Location C:\VIctory3
New-Item -ItemType Directory -Force C:\VIctory3\dump | Out-Null
$env:VICTORY_TRANSLATE_ONLY = $Hash
$env:VICTORY_EMIT_DUMP = "C:\VIctory3\dump"
$env:VICTORY_ABLATE = $null
.\target\diag\victory.exe translate --list $Source | Out-Null
$dxc = (Get-Command dxc.exe -ErrorAction SilentlyContinue).Source
if (-not $dxc) {
    $dxc = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Recurse -Filter dxc.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "x64" } | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $dxc) { Write-Host "dxc.exe not found"; exit }
Write-Host "dxc: $dxc"
Get-ChildItem C:\VIctory3\dump -Filter *.dxil | ForEach-Object {
    & $dxc -dumpbin $_.FullName | Out-File "$($_.FullName).txt" -Encoding utf8
    Write-Host "wrote $($_.Name).txt"
}
