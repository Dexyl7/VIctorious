# Dumps the shader's DXIL (and disassembly) with each of the three clusters
# that stop the crash left out, to compare against the full one.
param([string]$Hash = "a1266cc7391ae0d6")
Set-Location C:\VIctory3
$dxc = (Get-Command dxc.exe -ErrorAction SilentlyContinue).Source
if (-not $dxc) {
    $dxc = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Recurse -Filter dxc.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "x64" } | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
}
$env:VICTORY_TRANSLATE_ONLY = $Hash
$cases = @(
    @("getdim", "getDimensions", "23-26"),
    @("load", "rawBufferLoad", "161-176"),
    @("fmad", "tertiary", "311-797")
)
foreach ($c in $cases) {
    $dir = "C:\VIctory3\dump_$($c[0])"
    New-Item -ItemType Directory -Force $dir | Out-Null
    $env:VICTORY_EMIT_DUMP = $dir
    $env:VICTORY_ABLATE = $c[1]
    $env:VICTORY_ABLATE_RANGE = $c[2]
    .\target\diag\victory.exe translate --list "D:\Me Pirate\GTAVI\shaders.rpf" *> $null
    Get-ChildItem $dir -Filter *.dxil | ForEach-Object {
        & $dxc -dumpbin $_.FullName | Out-File "$($_.FullName).txt" -Encoding utf8
    }
    Write-Host "wrote $dir"
}
