# Runs the game with the crash skip list and one set of the fork's speed
# switches; the log goes to play_<Mode>.txt. Network off first.
param([ValidateSet("default", "relay", "relay1", "relay1card", "relaysafe", "fast", "fast2", "fast3", "smooth", "fast2si", "fast2mc", "fast2debug", "latewait", "both", "direct", "smooth2")][string]$Mode = "default", [int]$StageMs = 80, [switch]$NoDred, [switch]$Big, [switch]$Timing, [int]$Hoist = -1, [switch]$Feed, [switch]$Water, [switch]$Pool, [switch]$NoWater, [switch]$FeedAlt, [string]$Waits = "", [string]$Late = "", [switch]$NeverWait, [string]$Barriers = "", [ValidateSet("series-x", "series-s")][string]$Console = "series-x", [string]$Display = "", [string]$HeapPar = "", [string]$Fast = "", [string]$Plain = "", [switch]$Async, [string]$Shared = "", [string]$WideRefresh = "", [string]$WideThreads = "", [string]$CardMinKb = "", [string]$CardMb = "", [string]$NoBack = "", [string]$HeapPass = "", [switch]$Timeline, [string]$SyncBeside = "", [string]$SignalMeets = "", [string]$InPlace = "", [string]$LookupCache = "", [switch]$SlowTest, [string]$GpuClock = "", [switch]$Autopilot, [string]$RenderScale = "", [switch]$BvhMiss, [switch]$NoWatchdog, [string]$Sharpen = "", [switch]$BigMemory, [string]$TableStages = "", [switch]$HeapWorker, [switch]$NoHeapWorker, [string]$HeapMarked = "", [string]$AbsentCache = "", [switch]$PortConstants, [string]$PortDirect = "", [switch]$Profile, [string]$Ahead = "", [string]$SweepRoom = "", [string]$BufferPool = "")
Set-Location C:\VIctory3
$speed = @("VICTORY_RELAY", "VICTORY_LATE_WAIT", "VICTORY_WIDE_BACK", "VICTORY_CARD_BUFFERS_MIN_MB", "VICTORY_CARD_BUFFERS_MB",
           "VICTORY_PIPELINE_WORKERS", "VICTORY_ABLATE", "VICTORY_ABLATE_RANGE", "VICTORY_DXIL_FLAGS_OR",
           "VICTORY_NO_WAVE_SIZE", "VICTORY_SPLIT_FMAD", "VICTORY_SHADER_OPT", "VICTORY_TRANSLATE_ONLY", "VICTORY_WIDE_MERGE", "VICTORY_INDIRECT_WINDOW", "VICTORY_D3D12_DEBUG", "VICTORY_D3D12_GBV", "VICTORY_SETTLE_INDIRECT", "VICTORY_MERGE_CARD_ONLY", "VICTORY_TEXTURE_NO_WAIT", "VICTORY_TEXTURE_STAGE_MS", "VICTORY_TEXTURE_CATCHUP",
           "VICTORY_TRANSLATE_SKIP", "VICTORY_TRANSLATE_SKIP_FILE", "VICTORY_TRANSLATE_CS_ONLY", "VICTORY_TRANSLATE_TRACE",
           "VICTORY_POOL_COMMIT", "VICTORY_INPLACE", "VICTORY_INPLACE_BIG", "VICTORY_FAKE_STORE_HASH", "VICTORY_FAKE_STORE_VALUE", "VICTORY_GPU_TIMING", "VICTORY_CARD_REHASH_RUNS", "VICTORY_HOIST_CLAMPS", "VICTORY_GPU_FEED", "VICTORY_INDIRECT_WAITS", "VICTORY_FILL_NEVER_WAIT", "VICTORY_MERGE_RUNS", "VICTORY_MERGE_GAP", "VICTORY_SWEEP_MS", "VICTORY_TITLE_BARRIERS", "VICTORY_DISPLAY_MODE", "VICTORY_HEAP_SYNC_PARALLEL", "VICTORY_SEEN_FAST", "VICTORY_COPY_PLAIN", "VICTORY_BUFFERS_SHARED", "VICTORY_WIDE_REFRESH", "VICTORY_WIDE_THREADS", "VICTORY_NO_BACK", "VICTORY_HEAP_PASS_WIDE", "VICTORY_TIMELINE", "VICTORY_HEAP_SYNC_BESIDE", "VICTORY_SIGNAL_MEETS", "VICTORY_LOOKUP_CACHE", "VICTORY_SLOW_TEST", "VICTORY_GPU_CLOCK_SCALE", "VICTORY_AUTOPILOT_TEST", "VICTORY_COMPOSE", "VICTORY_OWN_UPSCALER", "VICTORY_RENDER_SCALE", "VICTORY_BVH_MISS", "VICTORY_SHARPEN", "VICTORY_TABLE_STAGES", "VICTORY_HEAP_WORKER", "VICTORY_HEAP_PASS_MARKED", "VICTORY_ABSENT_CACHE", "VICTORY_PORT_CONSTANTS", "VICTORY_PORT_DIRECT", "VICTORY_PROFILE", "VICTORY_PRESENTS_AHEAD", "VICTORY_SWEEP_FOLLOWS_ROOM", "VICTORY_BUFFER_POOL", "VICTORY_PROFILE_OUT", "VICTORY_TEXTURE_ROOM_PERCENT", "VICTORY_CARD_ROOM_FOR_TARGETS_MB", "VICTORY_SERIES_S_MEMORY", "VICTORY_NO_WATCHDOG", "VICTORY_TIMELINE_OUT", "VICTORY_ASYNC_PRESENT", "VICTORY_ASYNC_SUBMIT", "VICTORY_INDIRECT_GATHER", "VICTORY_CARD_BUFFERS_MIN_KB", "VICTORY_JOIN_PARTS", "VICTORY_BACK_OWN_VIEW", "VICTORY_TEXTURE_BACK")
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

# smooth2: smooth plus fxceless's switches of 2026-11-25 (indirect records gathered and clamped once
# a list, small buffers' copies on the card, parts joined, copy-back through our own view)
if ($Mode -eq "smooth2") { $env:VICTORY_INDIRECT_GATHER = "1"; $env:VICTORY_CARD_BUFFERS_MIN_KB = "1024"; $env:VICTORY_JOIN_PARTS = "1"; $env:VICTORY_BACK_OWN_VIEW = "1"; $env:VICTORY_TEXTURE_BACK = "1"; $env:VICTORY_LATE_WAIT = "3"; $env:VICTORY_INDIRECT_WAITS = "0"; $env:VICTORY_MERGE_RUNS = "4096"; $env:VICTORY_MERGE_GAP = "64"; $env:VICTORY_FILL_NEVER_WAIT = "1"; $env:VICTORY_SWEEP_MS = "10000"; $env:VICTORY_COPY_PLAIN = "1"; $env:VICTORY_ASYNC_SUBMIT = "1"; $env:VICTORY_ASYNC_PRESENT = "1"; $env:VICTORY_BUFFER_POOL = "1" }
if ($Mode -eq "smooth2" -and $Hoist -lt 0) { $env:VICTORY_HOIST_CLAMPS = "1" }
if ($Hoist -ge 0) { $env:VICTORY_HOIST_CLAMPS = "$Hoist" }
# -Water: textures shaders write that the title reads are copied back (pull request #999; on in smooth2)
if ($Water) { $env:VICTORY_TEXTURE_BACK = "1" }
# -Pool: the page pool committed in 1 GB sections, nothing else (to tell its cost from the feed's)
if ($NoWater) { Remove-Item Env:VICTORY_TEXTURE_BACK -ErrorAction SilentlyContinue }
if ($Pool) { $env:VICTORY_POOL_COMMIT = "1" }
if ($Async) { $env:VICTORY_ASYNC_SUBMIT = "1"; $env:VICTORY_ASYNC_PRESENT = "1" }
if ($Plain -ne "") { $env:VICTORY_COPY_PLAIN = $Plain }
if ($Shared -ne "") { $env:VICTORY_BUFFERS_SHARED = $Shared }
if ($WideRefresh -ne "") { $env:VICTORY_WIDE_REFRESH = $WideRefresh }
if ($WideThreads -ne "") { $env:VICTORY_WIDE_THREADS = $WideThreads }
if ($CardMinKb -ne "") { $env:VICTORY_CARD_BUFFERS_MIN_KB = $CardMinKb }
if ($CardMb -ne "") { $env:VICTORY_CARD_BUFFERS_MB = $CardMb }
if ($NoBack -ne "") { $env:VICTORY_NO_BACK = $NoBack }
if ($HeapPass -ne "") { $env:VICTORY_HEAP_PASS_WIDE = $HeapPass }
if ($SyncBeside -ne "") { $env:VICTORY_HEAP_SYNC_BESIDE = $SyncBeside }
if ($SignalMeets -ne "") { $env:VICTORY_SIGNAL_MEETS = $SignalMeets }
if ($InPlace -ne "") { $env:VICTORY_INPLACE = $InPlace; $env:VICTORY_POOL_COMMIT = "1" }
if ($LookupCache -ne "") { $env:VICTORY_LOOKUP_CACHE = $LookupCache }
if ($SlowTest) { $env:VICTORY_SLOW_TEST = "1" }
if ($GpuClock -ne "") { $env:VICTORY_GPU_CLOCK_SCALE = $GpuClock }
if ($Autopilot) { $env:VICTORY_AUTOPILOT_TEST = "1" }
# Series S: the interface plane laid over the game, and the title's own upscaler (FSR does not translate).
if ($Console -eq "series-s") { $env:VICTORY_COMPOSE = "1"; $env:VICTORY_OWN_UPSCALER = "1" }
# Series S at 720p: its render targets hold ~1.25 GB, not the 3 GB kept for them by default (measured on Series X):
# the rest goes to textures, which were refused for lack of room (black roads).
if ($Console -eq "series-s") { $env:VICTORY_CARD_ROOM_FOR_TARGETS_MB = "1536"; $env:VICTORY_TEXTURE_ROOM_PERCENT = "95" }
if ($RenderScale -ne "") { $env:VICTORY_RENDER_SCALE = $RenderScale }
if ($Sharpen -ne "") { $env:VICTORY_SHARPEN = $Sharpen }
if ($BigMemory) { $env:VICTORY_SERIES_S_MEMORY = "x" }
if ($Mode -eq "smooth2" -and $TableStages -eq "") { $env:VICTORY_TABLE_STAGES = "1" }
if ($TableStages -ne "") { $env:VICTORY_TABLE_STAGES = $TableStages }
if ($HeapWorker -or $Mode -eq "smooth2") { $env:VICTORY_HEAP_WORKER = "1" }
if ($NoHeapWorker) { Remove-Item Env:VICTORY_HEAP_WORKER -ErrorAction SilentlyContinue }
if ($HeapMarked -ne "") { $env:VICTORY_HEAP_PASS_MARKED = $HeapMarked }
if ($AbsentCache -ne "") { $env:VICTORY_ABSENT_CACHE = $AbsentCache }
if ($PortConstants) { $env:VICTORY_PORT_CONSTANTS = "1" }
if ($PortDirect -ne "") { $env:VICTORY_PORT_DIRECT = $PortDirect }
if ($BufferPool -ne "") { $env:VICTORY_BUFFER_POOL = $BufferPool }
if ($SweepRoom -ne "") { $env:VICTORY_SWEEP_FOLLOWS_ROOM = $SweepRoom }
if ($Ahead -ne "") { $env:VICTORY_PRESENTS_AHEAD = $Ahead }
if ($Profile) { $env:VICTORY_PROFILE = "1"; $env:VICTORY_PROFILE_OUT = "C:\VIctory3\profile.txt" }
if ($BvhMiss) { $env:VICTORY_BVH_MISS = "1" }
if ($NoWatchdog) { $env:VICTORY_NO_WATCHDOG = "1" }
if ($Timeline) { $env:VICTORY_TIMELINE = "1"; $env:VICTORY_TIMELINE_OUT = "C:\VIctory3\timeline.json" }
if ($Fast -ne "") { $env:VICTORY_SEEN_FAST = $Fast }
if ($HeapPar -ne "") { $env:VICTORY_HEAP_SYNC_PARALLEL = $HeapPar }
if ($Display -ne "") { $env:VICTORY_DISPLAY_MODE = $Display }
if ($Barriers -ne "") { $env:VICTORY_TITLE_BARRIERS = $Barriers }
if ($NeverWait) { $env:VICTORY_FILL_NEVER_WAIT = "1" }
if ($Late -ne "") { $env:VICTORY_LATE_WAIT = $Late }
if ($Waits -ne "") { $env:VICTORY_INDIRECT_WAITS = $Waits }
if ($FeedAlt) { $env:VICTORY_GPU_FEED = "alt"; $env:VICTORY_POOL_COMMIT = "1" }
if ($Feed) { $env:VICTORY_GPU_FEED = "1"; $env:VICTORY_POOL_COMMIT = "1" }
Write-Host "Mode: $Mode"
.\target\diag\victory.exe run "D:\Me Pirate\GTAVI\game_scarlett_master.exe" --execute --console $Console *> "C:\VIctory3\play_$Mode.txt"
Write-Host "Log: C:\VIctory3\play_$Mode.txt"
