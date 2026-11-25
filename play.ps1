# Runs the game with the crash skip list and one set of the fork's speed
# switches; the log goes to play_<Mode>.txt. Network off first.
param([ValidateSet("default", "relay", "relay1", "relay1card", "relaysafe", "fast", "fast2", "fast2si", "fast2mc", "fast2debug", "latewait", "both")][string]$Mode = "default")
Set-Location C:\VIctory3
$speed = @("VICTORY_RELAY", "VICTORY_LATE_WAIT", "VICTORY_WIDE_BACK", "VICTORY_CARD_BUFFERS_MIN_MB", "VICTORY_CARD_BUFFERS_MB",
           "VICTORY_PIPELINE_WORKERS", "VICTORY_ABLATE", "VICTORY_ABLATE_RANGE", "VICTORY_DXIL_FLAGS_OR",
           "VICTORY_NO_WAVE_SIZE", "VICTORY_SPLIT_FMAD", "VICTORY_SHADER_OPT", "VICTORY_TRANSLATE_ONLY", "VICTORY_WIDE_MERGE", "VICTORY_INDIRECT_WINDOW", "VICTORY_D3D12_DEBUG", "VICTORY_D3D12_GBV", "VICTORY_SETTLE_INDIRECT", "VICTORY_MERGE_CARD_ONLY",
           "VICTORY_TRANSLATE_SKIP", "VICTORY_TRANSLATE_SKIP_FILE", "VICTORY_TRANSLATE_CS_ONLY", "VICTORY_TRANSLATE_TRACE")
foreach ($k in $speed) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$env:VICTORY_SKIP_PIPELINES = "$(Get-Content C:\VIctory3\crashers_list.txt -Raw)".Trim()
if ($Mode -in @("relay", "both")) { $env:VICTORY_RELAY = "2" }
if ($Mode -in @("relay1", "relay1card", "relaysafe", "fast", "fast2", "fast2si", "fast2mc", "fast2debug")) { $env:VICTORY_RELAY = "1" }
if ($Mode -in @("relaysafe", "fast")) { $env:VICTORY_LATE_WAIT = "1"; $env:VICTORY_WIDE_MERGE = "1" }
if ($Mode -eq "fast") { $env:VICTORY_INDIRECT_WINDOW = "1" }
if ($Mode -in @("fast2", "fast2si", "fast2mc", "fast2debug")) { $env:VICTORY_LATE_WAIT = "2"; $env:VICTORY_WIDE_MERGE = "1"; $env:VICTORY_INDIRECT_WINDOW = "1" }
if ($Mode -eq "relay1card") {
    $env:VICTORY_WIDE_BACK = "1"; $env:VICTORY_CARD_BUFFERS_MIN_MB = "1"; $env:VICTORY_CARD_BUFFERS_MB = "1024"
}
if ($Mode -in @("latewait", "both")) {
    $env:VICTORY_LATE_WAIT = "3"; $env:VICTORY_WIDE_BACK = "1"
    $env:VICTORY_CARD_BUFFERS_MIN_MB = "1"; $env:VICTORY_CARD_BUFFERS_MB = "1024"
}
$env:VICTORY_D3D12_DRED = "1"
if ($Mode -eq "fast2mc") { $env:VICTORY_MERGE_CARD_ONLY = "1" }
if ($Mode -eq "fast2si") { $env:VICTORY_SETTLE_INDIRECT = "1" }
if ($Mode -eq "fast2debug") { $env:VICTORY_D3D12_DEBUG = "1"; $env:VICTORY_D3D12_GBV = "1" }
Write-Host "Mode: $Mode"
.\target\diag\victory.exe run "D:\Me Pirate\GTAVI\game_scarlett_master.exe" --execute *> "C:\VIctory3\play_$Mode.txt"
Write-Host "Log: C:\VIctory3\play_$Mode.txt"
