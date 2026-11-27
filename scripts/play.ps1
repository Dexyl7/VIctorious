# Starts the title with the settings this fork is played with (README.md, "How to run it").
#   scripts\play.ps1 -Exe <path of your own copy's executable>
# -Safe         without the newer, faster ways (the first thing to try when something is wrong)
# -RayTracing   the title's own ray-traced indirect light: lit interiors, about a third slower
# -Fsr4K        PC FidelityFX 4.1.2 at 3840x2160 (quality, balanced or performance; Series X), from Belobug and DexNom
# -Output       the title's output size (Series X): 2560x1440, 1920x1080, 1664x936 or 1280x720; with -Fsr4K, what FSR upscales to
# -FfxDir       the folder with AMD's amd_fidelityfx_upscaler_dx12.dll for -Fsr4K (C:\FidelityFX)
# -Build        target\diag (the default, the build all measurements are of) or target\release
# -Console      series-x (default) or series-s (less title memory: for PCs with 32 GB)
# -RenderScale  the title's render size as a share of the output, 0.2 to 1 (0.67: as Series S draws by default)
# -Sharpen      sharpening of the shown frame, 0 to 1 (0.6 suits a render scale of 0.67)
# -Ports        renderer patches 1, 5 and 6 (avatarmanz): constants heap and constant buffers read in place, guards' lock shared
# -More         other settings: -More @{VICTORY_PANEL='1'}
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
    [switch]$Ports
)
$settings = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'; VICTORY_D3D12_DRED = 'light'; VICTORY_SKIP_PIPELINES = 'none'
}
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    # VICTORY_THIN (list calls translated on threads of ours) is left off: on the RX 9070 XT it
    # gave single frames of stretched geometry (vertices thrown toward the camera), gone with it off.
    $settings.VICTORY_JOIN_PARTS = '8'; $settings.VICTORY_UPLOAD_AHEAD = '1'
}
# Series S shows its interface on a second plane laid over the game's (from dexyl7/VIctorious).
if ($Console -eq 'series-s') {
    $settings.VICTORY_COMPOSE = '1'
    # The title's own upscaler (FSR does not translate), and card room for textures:
    # Series S's targets hold about 1.25 GB, not the 3 GB kept for them by default.
    # Textures may use up to 94 % of the card's budget (this fork's default is 90; 75, VIctory3's value,
    # left 2-3 GB of the card unused while city roads and buildings waited black for room).
    $settings.VICTORY_OWN_UPSCALER = '1'; $settings.VICTORY_CARD_ROOM_FOR_TARGETS_MB = '1536'; $settings.VICTORY_TEXTURE_ROOM_PERCENT = '94'
}
if ($RenderScale -ne '') { $settings.VICTORY_RENDER_SCALE = $RenderScale }
if ($Sharpen -ne '') { $settings.VICTORY_SHARPEN = $Sharpen }
if ($RayTracing) { $settings.VICTORY_BVH_BUILDER = '1' }
if ($Fsr4K) {
    if ($Console -ne 'series-x') { Write-Host '-Fsr4K needs the Series X profile (leave -Console out).'; exit 1 }
    if (-not (Test-Path (Join-Path $FfxDir 'amd_fidelityfx_upscaler_dx12.dll'))) { Write-Host "No amd_fidelityfx_upscaler_dx12.dll in $FfxDir (-FfxDir)."; exit 1 }
    $settings.VICTORY_FSR_4K = $Fsr4K.ToLowerInvariant(); $settings.VICTORY_FIDELITYFX_DIR = $FfxDir
}
if ($Output -ne '') { $settings.VICTORY_OUTPUT = $Output }
if ($Ports) { $settings.VICTORY_PORT_CONSTANTS = '1'; $settings.VICTORY_PORT_CBV = '1'; $settings.VICTORY_PORT_GUARD = '1' }
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
