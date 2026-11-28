# Starts the title with the settings this fork is played with (README.md, "How to run it").
#   scripts\play.ps1 -Exe <path of your own copy's executable>
# -Safe         without the newer, faster ways (the first thing to try when something is wrong)
# -RayTracing   the title's own ray-traced indirect light: lit interiors, about a third slower
# -Fsr4K        PC FidelityFX 4.1.2 at 3840x2160 (quality, balanced or performance; Series X), from Belobug and DexNom
# -Output       the title's output size (Series X): 2560x1440, 1920x1080, 1664x936 or 1280x720; with -Fsr4K, what FSR upscales to
# -FfxDir       the folder with AMD's amd_fidelityfx_upscaler_dx12.dll for -Fsr4K (C:\FidelityFX)
# -SkipIntro    skips the logo and intro movies (VICTORY_SKIP_BINK=1; from C-GDev, fxceless PR 613, via CuteBooty)
# -FrameGen     with -Fsr4K: AMD frame generation (2x), from Vice-Tay 3236af66 after PastiiJay/klbf-lab 793c56c3;
#               needs amd_fidelityfx_framegeneration_dx12.dll and amd_fidelityfx_loader_dx12.dll in -FfxDir
# -Build        target\diag (the default, the build all measurements are of) or target\release
# -Console      series-x (default) or series-s (less title memory: for PCs with 32 GB)
# -RenderScale  the title's render size as a share of the output, 0.2 to 1 (0.67: as Series S draws by default)
# -Sharpen      sharpening of the shown frame, 0 to 1 (0.6 suits a render scale of 0.67)
# -NoPorts      without renderer patches 1, 5 and 6 (avatarmanz; on by default: constants heap and constant buffers read
#               in place, guards' lock shared; ~34 instead of ~43 ms a frame on Series S, lamps can flicker, trees pop)
# -More         other settings: -More @{VICTORY_THIN='0'}
# Run it from a normal window, not as administrator: the driver then keeps its shader cache elsewhere.
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$Safe,
    [switch]$RayTracing,
    [ValidateSet('quality', 'balanced', 'performance')][string]$Fsr4K,
    [string]$FfxDir = 'C:\FidelityFX',
    [ValidateSet('', '3840x2160', '2560x1440', '1920x1080', '1664x936', '1280x720')][string]$Output = '',
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [hashtable]$More = @{},
    [ValidateSet('series-x', 'series-s')][string]$Console = 'series-x',
    [string]$RenderScale = '',
    [string]$Sharpen = '',
    [switch]$Ports,
    [switch]$NoPorts,
    [switch]$FrameGen,
    [switch]$SkipIntro
)
$settings = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'; VICTORY_D3D12_DRED = 'light'; VICTORY_SKIP_PIPELINES = 'none'
    # Recording threads: a table's descriptor copies in one call; lookups forgotten only where buffers changed.
    VICTORY_TABLE_BATCH = '1'; VICTORY_FIND_BY_RANGE = '1'
    # Every title buffer of 64 KiB and more gets its copy on the card and is compared there (the way is
    # fxceless/victory-fork's, the KiB setting after its c36971b2; 1 MB was trustno1-coder/victory-nvidia's):
    # at 1 MB the snapshots went 4.0 -> 1.6 ms a frame, at 64 KiB to 0; 3,426 copies, 802 MB on the card,
    # frames mean 36.8 -> 33.6 ms, slowest tenth 38.6 -> 35.0 (2026-11-28).
    VICTORY_CARD_BUFFERS_MIN_KB = '64'
    # The heaps' records round asked for when the title submits, made while the line gets there:
    # the submitting thread's wait for it ~6-9 -> ~1-4 ms a frame (A/B drive, 2026-11-28).
    VICTORY_SYNC_AT_SUBMIT = '1'
    # Page mappings drain the line only if a submission in it reaches the memory mapped: cars no longer
    # vanish on coming close (their full models came too late), 2026-11-28.
    VICTORY_MAP_PRECISE = '1'
    # The ambient stand-in's reflection and glare patches (2026-11-28) are left off: never confirmed in play.
    VICTORY_AMBIENT_SPECULAR = '0'; VICTORY_AMBIENT_OTHER = '0'
}
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    # VICTORY_THIN: list calls translated on threads of ours, not the title's (frames about 20 %
    # shorter here). It had single frames of stretched geometry; the queue's page mappings now wait
    # for the line first (VICTORY_MAP_DRAIN, on); some geometry flicker may remain. -More @{VICTORY_THIN='0'}: off.
    $settings.VICTORY_THIN = '1'
    $settings.VICTORY_JOIN_PARTS = '8'; $settings.VICTORY_UPLOAD_AHEAD = '1'
}
# Series S shows its interface on a second plane laid over the game's (from randomguy12000/VIctorious).
if ($Console -eq 'series-s') {
    $settings.VICTORY_COMPOSE = '1'
    # The title's own upscaler (FSR does not translate), and card room for textures:
    # Series S's targets hold about 1.25 GB, not the 3 GB kept for them by default.
    # Textures may use up to 94 % of the card's budget (this fork's default is 90; 75, VIctory3's value,
    # left 2-3 GB of the card unused while city roads and buildings waited black for room).
    $settings.VICTORY_OWN_UPSCALER = '1'; $settings.VICTORY_CARD_ROOM_FOR_TARGETS_MB = '1536'; $settings.VICTORY_TEXTURE_ROOM_PERCENT = '94'
}
# Windows of title memory (host copies shaders read, about 3 GB) in the card's memory, written by the CPU
# through the resizable BAR (Smart Access Memory) instead of kept in the PC's memory: the PC had 1.8 GB free
# a minute in and under 0.2 GB at 16 minutes (12 fps, audio lagging) while the card had 7 GB unused. Only while
# a fifth of the card's budget is free, so textures keep their room; without BAR support nothing changes.
$settings.VICTORY_WINDOWS_BAR_MB = '3584'
# Windows of title memory in the title's private memory read in place (crate::inplace stage 1): no host copy, nothing
# to bring up to date; windows over its page pool (views of a reserved section) are copied as before.
$settings.VICTORY_INPLACE = '1'
if ($RenderScale -ne '') { $settings.VICTORY_RENDER_SCALE = $RenderScale }
if ($Sharpen -ne '') { $settings.VICTORY_SHARPEN = $Sharpen }
if ($RayTracing) { $settings.VICTORY_BVH_BUILDER = '1' }
if ($Fsr4K) {
    if ($Console -ne 'series-x') { Write-Host '-Fsr4K needs the Series X profile (leave -Console out).'; exit 1 }
    if (-not (Test-Path (Join-Path $FfxDir 'amd_fidelityfx_upscaler_dx12.dll'))) { Write-Host "No amd_fidelityfx_upscaler_dx12.dll in $FfxDir (-FfxDir)."; exit 1 }
    $settings.VICTORY_FSR_4K = $Fsr4K.ToLowerInvariant(); $settings.VICTORY_FIDELITYFX_DIR = $FfxDir
    if ($FrameGen) {
        foreach ($dll in 'amd_fidelityfx_framegeneration_dx12.dll', 'amd_fidelityfx_loader_dx12.dll') {
            if (-not (Test-Path (Join-Path $FfxDir $dll))) { Write-Host "No $dll in $FfxDir (-FfxDir): -FrameGen needs it."; exit 1 }
        }
        $settings.VICTORY_FRAMEGEN = '1'; $settings.VICTORY_FFX_FG_DIR = $FfxDir
    }
}
if ($SkipIntro) { $settings.VICTORY_SKIP_BINK = '1' }
if ($FrameGen -and -not $Fsr4K) { Write-Host '-FrameGen needs -Fsr4K (it takes the upscaler''s depth and motion).'; exit 1 }
if ($Output -ne '') { $settings.VICTORY_OUTPUT = $Output }
# Patch 1's ordering (each in-place indirect execution checked alone, after a full wait of the card)
# cost about 36 ms of the card's time a frame here (timelines tl2: 61 ms busy with it, 25 without);
# without it trees and palms can drop out for a few frames. -More @{VICTORY_PORT_CONSTANTS_ORDER='1'} brings it back.
if (-not $NoPorts) { $settings.VICTORY_PORT_CONSTANTS = '1'; $settings.VICTORY_PORT_CBV = '1'; $settings.VICTORY_PORT_GUARD = '1'; $settings.VICTORY_PORT_CONSTANTS_ORDER = '0' }
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }

$victory = Join-Path $PSScriptRoot "..\target\$Build\victory.exe"
if (-not (Test-Path $victory)) { Write-Host "No $victory. Build it first (README.md)."; return }
if (-not (Test-Path $Exe)) { Write-Host "No $Exe."; return }
# (Other settings left over in this window go; where your files are stays.)
$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR', 'VICTORY_FIDELITYFX_DIR'
Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths } | ForEach-Object { Remove-Item "Env:$($_.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
$Exe = (Resolve-Path $Exe).Path
Push-Location (Join-Path $PSScriptRoot '..')   # victory.toml is looked for here
try { & $victory run $Exe --execute --console $Console } finally { Pop-Location }
