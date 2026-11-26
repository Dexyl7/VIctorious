# Runs the game with the crash skip list and one set of the fork's speed
# switches; the log goes to play_<Mode>.txt. Network off first.
param([ValidateSet("default", "relay", "relay1", "relay1card", "relaysafe", "fast", "fast2", "fast3", "smooth", "fast2si", "fast2mc", "fast2debug", "latewait", "both", "direct", "smooth2")][string]$Mode = "default", [int]$StageMs = 80, [switch]$NoDred, [switch]$Big, [switch]$Timing, [int]$Hoist = 0, [switch]$Feed, [switch]$Water, [switch]$Pool, [switch]$NoWater, [switch]$FeedAlt, [string]$Waits = "", [string]$Late = "", [switch]$NeverWait)
Set-Location C:\VIctory3
$speed = @("VICTORY_RELAY", "VICTORY_LATE_WAIT", "VICTORY_WIDE_BACK", "VICTORY_CARD_BUFFERS_MIN_MB", "VICTORY_CARD_BUFFERS_MB",
           "VICTORY_PIPELINE_WORKERS", "VICTORY_ABLATE", "VICTORY_ABLATE_RANGE", "VICTORY_DXIL_FLAGS_OR",
           "VICTORY_NO_WAVE_SIZE", "VICTORY_SPLIT_FMAD", "VICTORY_SHADER_OPT", "VICTORY_TRANSLATE_ONLY", "VICTORY_WIDE_MERGE", "VICTORY_INDIRECT_WINDOW", "VICTORY_D3D12_DEBUG", "VICTORY_D3D12_GBV", "VICTORY_SETTLE_INDIRECT", "VICTORY_MERGE_CARD_ONLY", "VICTORY_TEXTURE_NO_WAIT", "VICTORY_TEXTURE_STAGE_MS", "VICTORY_TEXTURE_CATCHUP",
           "VICTORY_TRANSLATE_SKIP", "VICTORY_TRANSLATE_SKIP_FILE", "VICTORY_TRANSLATE_CS_ONLY", "VICTORY_TRANSLATE_TRACE",
           "VICTORY_POOL_COMMIT", "VICTORY_INPLACE", "VICTORY_INPLACE_BIG", "VICTORY_FAKE_STORE_HASH", "VICTORY_FAKE_STORE_VALUE", "VICTORY_GPU_TIMING", "VICTORY_CARD_REHASH_RUNS", "VICTORY_HOIST_CLAMPS", "VICTORY_GPU_FEED", "VICTORY_INDIRECT_WAITS", "VICTORY_FILL_NEVER_WAIT", "VICTORY_MERGE_RUNS", "VICTORY_MERGE_GAP", "VICTORY_SWEEP_MS", "VICTORY_INDIRECT_GATHER", "VICTORY_CARD_BUFFERS_MIN_KB", "VICTORY_JOIN_PARTS", "VICTORY_BACK_OWN_VIEW", "VICTORY_TEXTURE_BACK")
foreach ($k in $speed) { Remove-Item "Env:$k" -ErrorAction SilentlyContinue }
$env:VICTORY_SKIP_PIPELINES = "$(Get-Content C:\VIctory3\crashers_list.txt -Raw)".Trim()
if ($Mode -in @("relay", "both")) { $env:VICTORY_RELAY = "2" }
if ($Mode -in @("relay1", "relay1card", "relaysafe", "fast", "fast2", "fast3", "smooth", "smooth2", "fast2si", "fast2mc", "fast2debug", "direct")) { $env:VICTORY_RELAY = "1" }
if ($Mode -in @("relaysafe", "fast")) { $env:VICTORY_LATE_WAIT = "1"; $env:VICTORY_WIDE_MERGE = "1" }
if ($Mode -eq "fast") { $env:VICTORY_INDIRECT_WINDOW = "1" }
if ($Mode -in @("fast2", "fast3", "smooth", "smooth2", "fast2si", "fast2mc", "fast2debug")) { $env:VICTORY_LATE_WAIT = "2"; $env:VICTORY_WIDE_MERGE = "1"; $env:VICTORY_INDIRECT_WINDOW = "1" }
if ($Mode -eq "relay1card") {
    $env:VICTORY_WIDE_BACK = "1"; $env:VICTORY_CARD_BUFFERS_MIN_MB = "1"; $env:VICTORY_CARD_BUFFERS_MB = "1024"
}
if ($Mode -in @("latewait", "both")) {
    $env:VICTORY_LATE_WAIT = "3"; $env:VICTORY_WIDE_BACK = "1"
    $env:VICTORY_CARD_BUFFERS_MIN_MB = "1"; $env:VICTORY_CARD_BUFFERS_MB = "1024"
}
if (-not $NoDred) { $env:VICTORY_D3D12_DRED = "1" } else { Remove-Item Env:VICTORY_D3D12_DRED -ErrorAction SilentlyContinue }
# direct: smooth without late wait (in-place memory is not safe with it), the page pool committed and title memory in place
if ($Mode -eq "direct") { $env:VICTORY_WIDE_MERGE = "1"; $env:VICTORY_INDIRECT_WINDOW = "1"; $env:VICTORY_POOL_COMMIT = "1"; $env:VICTORY_INPLACE = "3"; if ($Big) { $env:VICTORY_INPLACE_BIG = "1" } }
if ($Mode -in @("fast3", "smooth", "smooth2", "direct")) { $env:VICTORY_TEXTURE_NO_WAIT = "1" }
if ($Mode -in @("smooth", "smooth2", "direct")) { $env:VICTORY_TEXTURE_STAGE_MS = "$StageMs"; $env:VICTORY_TEXTURE_CATCHUP = "0" }
if ($Mode -eq "fast2mc") { $env:VICTORY_MERGE_CARD_ONLY = "1" }
if ($Mode -eq "fast2si") { $env:VICTORY_SETTLE_INDIRECT = "1" }
if ($Mode -eq "fast2debug") { $env:VICTORY_D3D12_DEBUG = "1"; $env:VICTORY_D3D12_GBV = "1" }
if ($Timing) { $env:VICTORY_GPU_TIMING = "1" }
if ($Hoist -gt 0) { $env:VICTORY_HOIST_CLAMPS = "$Hoist" }
# smooth2: smooth plus fxceless's switches of 2026-11-25 (indirect records gathered and clamped once
# a list, small buffers' copies on the card, parts joined, copy-back through our own view)
if ($Mode -eq "smooth2") { $env:VICTORY_INDIRECT_GATHER = "1"; $env:VICTORY_CARD_BUFFERS_MIN_KB = "64"; $env:VICTORY_JOIN_PARTS = "1"; $env:VICTORY_BACK_OWN_VIEW = "1"; $env:VICTORY_TEXTURE_BACK = "1"; $env:VICTORY_LATE_WAIT = "3"; $env:VICTORY_INDIRECT_WAITS = "0"; $env:VICTORY_MERGE_RUNS = "4096"; $env:VICTORY_MERGE_GAP = "64"; $env:VICTORY_FILL_NEVER_WAIT = "1"; $env:VICTORY_SWEEP_MS = "10000" }
# -Water: textures shaders write that the title reads are copied back (pull request #999; on in smooth2)
if ($Water) { $env:VICTORY_TEXTURE_BACK = "1" }
# -Pool: the page pool committed in 1 GB sections, nothing else (to tell its cost from the feed's)
if ($NoWater) { Remove-Item Env:VICTORY_TEXTURE_BACK -ErrorAction SilentlyContinue }
if ($Pool) { $env:VICTORY_POOL_COMMIT = "1" }
if ($NeverWait) { $env:VICTORY_FILL_NEVER_WAIT = "1" }
if ($Late -ne "") { $env:VICTORY_LATE_WAIT = $Late }
if ($Waits -ne "") { $env:VICTORY_INDIRECT_WAITS = $Waits }
if ($FeedAlt) { $env:VICTORY_GPU_FEED = "alt"; $env:VICTORY_POOL_COMMIT = "1" }
if ($Feed) { $env:VICTORY_GPU_FEED = "1"; $env:VICTORY_POOL_COMMIT = "1" }
Write-Host "Mode: $Mode"
.\target\diag\victory.exe run "D:\Me Pirate\GTAVI\game_scarlett_master.exe" --execute *> "C:\VIctory3\play_$Mode.txt"
Write-Host "Log: C:\VIctory3\play_$Mode.txt"
