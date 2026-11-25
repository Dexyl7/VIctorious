# Narrows down which DXIL operation in one crashing compute shader makes the
# AMD driver compiler (amdxc64.dll) crash. Builds that one shader's pipeline
# offline (victory translate --pipelines) many times with operations left out.
# No game run; no network needed. Results: C:\VIctory3\bisect_results.txt
param(
    [string]$Hash = "a1266cc7391ae0d6",
    [string[]]$Groups = @("unary", "tertiary", "getDimensions", "rawBufferLoad")
)
Set-Location C:\VIctory3
foreach ($k in @("VICTORY_TRANSLATE_SKIP", "VICTORY_TRANSLATE_SKIP_FILE", "VICTORY_TRANSLATE_CS_ONLY", "VICTORY_TRANSLATE_TRACE", "VICTORY_EMIT_DUMP")) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$exe = "C:\VIctory3\target\diag\victory.exe"
$rpf = "D:\Me Pirate\GTAVI\shaders.rpf"
$log = "C:\VIctory3\bisect_results.txt"
$env:VICTORY_TRANSLATE_ONLY = $Hash
"Shader $Hash" | Out-File $log -Encoding utf8
function Say($t) { Write-Host $t; $t | Out-File $log -Append -Encoding utf8 }

# Builds the pipeline with $ablate (and $range) left out: $true if the driver crashed.
function Crashes($ablate, $range) {
    $env:VICTORY_ABLATE = $ablate
    $env:VICTORY_ABLATE_RANGE = $range
    & $exe translate --pipelines $rpf *> C:\VIctory3\bisect_out.txt
    return ($LASTEXITCODE -eq -1073741819)
}
# How many operations $ablate matches (translation only, no driver).
function Count($ablate) {
    $env:VICTORY_ABLATE = $ablate
    $env:VICTORY_ABLATE_RANGE = "0-0"
    $text = & $exe translate $rpf 2>&1 | Out-String
    if ($text -match "ablated\s+(\d+) matches") { return [int]$Matches[1] }
    return 0
}

if (-not (Crashes "" "")) { Say "Baseline did NOT crash: nothing to find. Stopping."; exit }
Say "Baseline crashes, as expected."

foreach ($g in $Groups) {
    $n = Count $g
    Say ""
    Say "== $g : $n operations"
    if ($n -eq 0) { continue }
    if (Crashes $g "0-$n") { Say "  leaving all of them out still crashes: not this group."; continue }
    # Smallest k: leaving out 0..k-1 stops the crash.
    $lo = 1; $hi = $n
    while ($lo -lt $hi) {
        $mid = [int][math]::Floor(($lo + $hi) / 2)
        if (Crashes $g "0-$mid") { $lo = $mid + 1 } else { $hi = $mid }
        Write-Host "    prefix search: $lo..$hi"
    }
    $k = $lo
    # Largest j: leaving out j..k-1 still stops it.
    $lo = 0; $hi = $k - 1
    while ($lo -lt $hi) {
        $mid = [int][math]::Ceiling(($lo + $hi) / 2)
        if (Crashes $g "$mid-$k") { $hi = $mid - 1 } else { $lo = $mid }
        Write-Host "    start search: $lo..$hi"
    }
    $j = $lo
    $single = -not (Crashes $g "$($k-1)-$k")
    Say "  leaving out $g operations $j to $($k-1) ($($k-$j) of them) stops the crash"
    Say "  leaving out only operation $($k-1) alone stops it: $single"
    # Name the operations in that range (printed when 8 or fewer).
    $s = $j; if ($k - $s -gt 8) { $s = $k - 8 }
    $null = Crashes $g "$s-$k"
    Get-Content C:\VIctory3\bisect_out.txt | Select-String "ablated #" | ForEach-Object { Say "    $_" }
}
Say ""
Say "Done."
