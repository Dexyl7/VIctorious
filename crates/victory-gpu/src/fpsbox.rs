//! The frame rate, drawn into the top left corner of every frame shown.
//! (From dexyl7/VIctorious; memory lines after Belobug's panel, PR #348.)
//!
//! The window's caption has the frame rate, but a capture of the window's
//! picture (a stream) does not carry the caption. Here the number is part
//! of the picture: a small box of text copied onto the swap chain's back
//! buffer right after the title's frame, before it is presented. The
//! title's own frame is not touched, so saved frames and everything read
//! back from it are as before.
//!
//! The text is drawn on the CPU with a font of its own (5 by 7 dots a
//! glyph) into a buffer the card copies from; a buffer is made once for a
//! text and never written again, so a present still in the air reads what
//! it was recorded with. On by default; `VICTORY_FPS_COUNTER=0` leaves the
//! frame as the title drew it.
//!
//! Under it, two lines of memory (`VICTORY_FPS_MEMORY=0`: none): the card's
//! own memory in use against the budget the system gives this process, and
//! the PC's memory the card uses and the PC's memory free. The card's own
//! near its budget and the PC's free near nothing is when Windows starts to
//! page, and the game freezes (after Belobug's panel, pull request #348).
//!
//! In the top right corner, the time since the first frame was shown, as
//! minutes and seconds (hours once past one), so a run's length is on the
//! picture (`VICTORY_FPS_CLOCK=0`: none).

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;

/// The frame rate in tenths of a frame a second, as last measured (once a
/// second, by the presenter's meter); `u32::MAX` before the first.
static TENTHS: AtomicU32 = AtomicU32::new(u32::MAX);

/// The memory lines' numbers in tenths of a gigabyte: the card's own memory
/// in use and its budget, the PC's memory the card uses, the PC's memory
/// free; `u32::MAX` before the first look.
static MEMORY: [AtomicU32; 4] = [const { AtomicU32::new(u32::MAX) }; 4];

/// Whether the memory lines are drawn.
fn memory_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_FPS_MEMORY").is_none_or(|v| v != "0"))
}

/// The memory just measured, in bytes: the card's own in use and its
/// budget, the PC's the card uses, the PC's free.
pub(crate) fn set_memory(bytes: [u64; 4]) {
    for (at, b) in MEMORY.iter().zip(bytes) {
        at.store(((b as f64) / (1u64 << 30) as f64 * 10.0).round().min(9_999.0) as u32, Ordering::Relaxed);
    }
}

/// The memory lines for numbers in tenths of a gigabyte.
fn memory_text(m: [u32; 4]) -> [String; 2] {
    let gb = |t: u32| format!("{}.{}", t / 10, t % 10);
    [format!("VRAM {} OF {} GB", gb(m[0]), gb(m[1])), format!("SHARED {} GB FREE RAM {} GB", gb(m[2]), gb(m[3]))]
}

/// Whether the clock is drawn.
fn clock_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_FPS_CLOCK").is_none_or(|v| v != "0"))
}

/// When the first frame was shown.
static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// The clock's text for `seconds` gone: M:SS, or H:MM:SS past an hour.
fn clock_text(seconds: u64) -> String {
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// Whether the counter is drawn.
fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_FPS_COUNTER").is_none_or(|v| v != "0"))
}

/// The frame rate just measured.
pub(crate) fn set(fps: f64) {
    if fps.is_finite() && fps >= 0.0 {
        TENTHS.store((fps * 10.0).round().min(99_999.0) as u32, Ordering::Relaxed);
    }
}

/// A glyph's seven rows, five dots each (bit 4 is the left one).
fn glyph(c: char) -> [u8; 7] {
    match c {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        _ => [0; 7],
    }
}

/// The text for a frame rate in tenths, with the frame time it means.
fn text(tenths: u32) -> String {
    let ms = if tenths == 0 { 0 } else { (10_000 + tenths / 2) / tenths };
    format!("{}.{} FPS {} MS", tenths / 10, tenths % 10, ms)
}

/// Bytes a pixel of a swap chain format this layer presents in, or none.
fn pixel_bytes(format: DXGI_FORMAT) -> Option<usize> {
    match format {
        DXGI_FORMAT_R8G8B8A8_UNORM | DXGI_FORMAT_B8G8R8A8_UNORM | DXGI_FORMAT_R10G10B10A2_UNORM => Some(4),
        DXGI_FORMAT_R16G16B16A16_FLOAT => Some(8),
        _ => None,
    }
}

/// One pixel of `format`: of the text (`lit`) or of the box behind it.
fn pixel(format: DXGI_FORMAT, lit: bool, out: &mut [u8]) {
    // The text: a bright yellow. The box: nearly black.
    let (r, g, b): (f32, f32, f32) = if lit { (1.0, 0.92, 0.16) } else { (0.03, 0.03, 0.03) };
    let byte = |v: f32| (v * 255.0).round() as u8;
    match format {
        DXGI_FORMAT_R8G8B8A8_UNORM => out.copy_from_slice(&[byte(r), byte(g), byte(b), 255]),
        DXGI_FORMAT_B8G8R8A8_UNORM => out.copy_from_slice(&[byte(b), byte(g), byte(r), 255]),
        DXGI_FORMAT_R10G10B10A2_UNORM => {
            let ten = |v: f32| (v * 1023.0).round() as u32;
            out.copy_from_slice(&(ten(r) | ten(g) << 10 | ten(b) << 20 | 3 << 30).to_le_bytes());
        }
        _ => {
            for (i, v) in [r, g, b, 1.0].into_iter().enumerate() {
                out[i * 2..i * 2 + 2].copy_from_slice(&half(v).to_le_bytes());
            }
        }
    }
}

/// A value from 0 to 1 as a 16-bit float.
fn half(v: f32) -> u16 {
    let bits = v.clamp(0.0, 1.0).to_bits();
    if v <= 0.0 {
        return 0;
    }
    // Sign 0; the exponent moves from a bias of 127 to one of 15; the top
    // ten bits of the fraction stay. Values under 2^-14 (none of ours) are 0.
    let exponent = ((bits >> 23) & 0xFF) as i32 - 127 + 15;
    if exponent <= 0 {
        return 0;
    }
    ((exponent as u32) << 10 | (bits >> 13) & 0x3FF) as u16
}

/// The box for `text` at `scale` dots a pixel: its width and height in
/// pixels and, row by row from the top, whether each pixel is text.
fn bitmap(text: &str, scale: u32) -> (u32, u32, Vec<bool>) {
    let chars: Vec<char> = text.chars().collect();
    let pad = 2 * scale;
    let width = 2 * pad + chars.len() as u32 * 6 * scale - scale;
    let height = 2 * pad + 7 * scale;
    let mut lit = vec![false; (width * height) as usize];
    for (i, &c) in chars.iter().enumerate() {
        let rows = glyph(c);
        for (row, bits) in rows.into_iter().enumerate() {
            for column in 0..5u32 {
                if bits >> (4 - column) & 1 == 0 {
                    continue;
                }
                let (x0, y0) = (pad + (i as u32 * 6 + column) * scale, pad + row as u32 * scale);
                for y in y0..y0 + scale {
                    for x in x0..x0 + scale {
                        lit[(y * width + x) as usize] = true;
                    }
                }
            }
        }
    }
    (width, height, lit)
}

/// A buffer the card copies a text's box from, with its footprint.
struct Made {
    buffer: ID3D12Resource,
    width: u32,
    height: u32,
    pitch: u32,
}

// SAFETY: a free-threaded D3D12 object and plain numbers.
unsafe impl Send for Made {}

/// The boxes made so far, by text, format and scale.
static MADE: Mutex<Option<HashMap<(String, i32, u32), Made>>> = Mutex::new(None);

/// The clock's box: a new text every second, so only the last is kept and
/// the one before goes to the host's retired objects.
static CLOCK: Mutex<Option<((String, i32, u32), Made)>> = Mutex::new(None);

/// A buffer holding the box for `text` at `scale` in `format`.
fn make(host: &crate::host::Host, text: &str, format: DXGI_FORMAT, scale: u32, bytes: usize) -> Option<Made> {
    let (w, h, lit) = bitmap(text, scale);
    let pitch = (w as usize * bytes).next_multiple_of(D3D12_TEXTURE_DATA_PITCH_ALIGNMENT as usize);
    let (buffer, data) = host.shared_buffer((pitch * h as usize) as u64)?;
    // SAFETY: the buffer's own mapped bytes, `pitch * h` of them.
    let out = unsafe { std::slice::from_raw_parts_mut(data, pitch * h as usize) };
    for yy in 0..h as usize {
        for x in 0..w as usize {
            let at = yy * pitch + x * bytes;
            pixel(format, lit[yy * w as usize + x], &mut out[at..at + bytes]);
        }
    }
    Some(Made { buffer, width: w, height: h, pitch: pitch as u32 })
}

/// Records the copy of `m`'s box onto `back` at `x`, `y`.
fn copy(list: &ID3D12GraphicsCommandList, back: &ID3D12Resource, format: DXGI_FORMAT, m: &Made, x: u32, y: u32) {
    // SAFETY: an open list; the back buffer is a copy's destination in it
    // and the caller checked the box lies inside it; our buffer holds
    // `pitch * height` bytes and is kept until it is retired.
    unsafe {
        let from = D3D12_TEXTURE_COPY_LOCATION {
            pResource: std::mem::transmute_copy(&m.buffer),
            Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                    Offset: 0,
                    Footprint: D3D12_SUBRESOURCE_FOOTPRINT { Format: format, Width: m.width, Height: m.height, Depth: 1, RowPitch: m.pitch },
                },
            },
        };
        let to = D3D12_TEXTURE_COPY_LOCATION {
            pResource: std::mem::transmute_copy(back),
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
        };
        list.CopyTextureRegion(&to, x, y, 0, &from, None);
    }
}

/// The clock in the top right corner, `margin` in from the edges.
fn draw_clock(host: &crate::host::Host, list: &ID3D12GraphicsCommandList, back: &ID3D12Resource, (width, height, format): (u32, u32, DXGI_FORMAT), scale: u32, bytes: usize) {
    let started = *STARTED.get_or_init(std::time::Instant::now);
    let key = (clock_text(started.elapsed().as_secs()), format.0, scale);
    let mut clock = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    if clock.as_ref().is_none_or(|(k, _)| *k != key) {
        let Some(m) = make(host, &key.0, format, scale, bytes) else {
            return;
        };
        if let Some((_, old)) = clock.replace((key, m)) {
            crate::host::retire(old.buffer);
        }
    }
    let Some((_, m)) = clock.as_ref() else {
        return;
    };
    let margin = 4 * scale;
    if m.width + 2 * margin > width || m.height + 2 * margin > height {
        return;
    }
    copy(list, back, format, m, width - margin - m.width, margin);
}

/// Copies the frame rate's box onto `back`, a back buffer of `width` by
/// `height` pixels of `format` that `list` holds as a copy's destination.
/// Nothing if the counter is off, no rate was measured yet, the format is
/// not one of ours or the frame is too small for the box.
pub(crate) fn draw(host: &crate::host::Host, list: &ID3D12GraphicsCommandList, back: &ID3D12Resource, (width, height, format): (u32, u32, DXGI_FORMAT)) {
    if !on() {
        return;
    }
    STARTED.get_or_init(std::time::Instant::now);
    let tenths = TENTHS.load(Ordering::Relaxed);
    let Some(bytes) = pixel_bytes(format).filter(|_| tenths != u32::MAX) else {
        return;
    };
    // About a fortieth of the frame's height a glyph: 6 dots a pixel at
    // 2160 lines, 3 at 1080.
    let scale = (height / 360).clamp(2, 12);
    if clock_on() {
        draw_clock(host, list, back, (width, height, format), scale, bytes);
    }
    let mut lines = vec![text(tenths)];
    let m = MEMORY.each_ref().map(|a| a.load(Ordering::Relaxed));
    if memory_on() && !m.contains(&u32::MAX) {
        lines.extend(memory_text(m));
    }
    let mut made = MADE.lock().unwrap_or_else(|e| e.into_inner());
    let made = made.get_or_insert_with(HashMap::new);
    if made.len() > 512 {
        // (Every text is a buffer of a few hundred KiB; a rate that wanders
        // over hundreds of values is not expected.) A copy still in the air
        // may read one: the buffers go to the host's retired objects, freed
        // once the card is past them.
        for (_, m) in made.drain() {
            crate::host::retire(m.buffer);
        }
    }
    let margin = 4 * scale;
    let mut y = margin;
    for line in lines {
        let key = (line, format.0, scale);
        if !made.contains_key(&key) {
            let Some(m) = make(host, &key.0, format, scale, bytes) else {
                return;
            };
            made.insert(key.clone(), m);
        }
        let Some(m) = made.get(&key) else {
            return;
        };
        if m.width + 2 * margin > width || y + m.height + margin > height {
            return;
        }
        copy(list, back, format, m, margin, y);
        y += m.height + scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_lines_read_in_tenths_of_a_gigabyte_with_every_glyph_drawn() {
        let lines = memory_text([95, 131, 70, 20]);
        assert_eq!(lines[0], "VRAM 9.5 OF 13.1 GB");
        assert_eq!(lines[1], "SHARED 7.0 GB FREE RAM 2.0 GB");
        for c in lines.concat().chars().filter(|&c| c != ' ') {
            assert!(glyph(c).iter().any(|&r| r != 0), "{c}");
        }
    }

    #[test]
    fn the_clock_reads_minutes_and_seconds_then_hours() {
        assert_eq!(clock_text(0), "0:00");
        assert_eq!(clock_text(75), "1:15");
        assert_eq!(clock_text(3599), "59:59");
        assert_eq!(clock_text(3600 + 62), "1:01:02");
        assert!(glyph(':').iter().any(|&r| r != 0));
    }

    #[test]
    fn the_text_is_the_rate_to_a_tenth() {
        assert_eq!(text(103), "10.3 FPS 97 MS");
        assert_eq!(text(7), "0.7 FPS 1429 MS");
        assert_eq!(text(300), "30.0 FPS 33 MS");
    }

    #[test]
    fn the_box_holds_every_glyph_with_a_margin() {
        let (w, h, lit) = bitmap("10.3 FPS", 3);
        // Eight glyphs of five dots and seven gaps of one, a pad of two dots all round.
        assert_eq!((w, h), ((8 * 5 + 7 + 4) * 3, (7 + 4) * 3));
        assert_eq!(lit.len(), (w * h) as usize);
        // The pad is box, not text; the "1" has its stem lit.
        assert!(lit[..(w * 6) as usize].iter().all(|&l| !l), "the top pad");
        assert!((0..h).all(|y| !lit[(y * w) as usize] && !lit[(y * w + w - 1) as usize]), "the side pads");
        let at = |x: u32, y: u32| lit[(y * w + x) as usize];
        // Glyph 0 ("1"): dot column 2 of row 0 is lit, column 0 is not.
        assert!(at(6 + 2 * 3, 6) && !at(6, 6));
        // A space lights nothing: glyph 4.
        assert!((0..7 * 3).all(|y| (0..5 * 3).all(|x| !at(6 + 4 * 18 + x, 6 + y))));
        // Every glyph used has at least one dot, but the space.
        for c in "0123456789.FPSM".chars() {
            assert!(glyph(c).iter().any(|&r| r != 0), "{c}");
        }
    }

    #[test]
    fn pixels_are_in_the_formats_the_layer_presents_in() {
        let mut four = [0u8; 4];
        pixel(DXGI_FORMAT_R8G8B8A8_UNORM, true, &mut four);
        assert_eq!(four, [255, 235, 41, 255]);
        pixel(DXGI_FORMAT_B8G8R8A8_UNORM, true, &mut four);
        assert_eq!(four, [41, 235, 255, 255]);
        pixel(DXGI_FORMAT_R10G10B10A2_UNORM, true, &mut four);
        let v = u32::from_le_bytes(four);
        assert_eq!((v & 1023, v >> 10 & 1023, v >> 20 & 1023, v >> 30), (1023, 941, 164, 3));
        let mut eight = [0u8; 8];
        pixel(DXGI_FORMAT_R16G16B16A16_FLOAT, true, &mut eight);
        assert_eq!(u16::from_le_bytes([eight[0], eight[1]]), 0x3C00, "1.0 as a half");
        assert_eq!(u16::from_le_bytes([eight[6], eight[7]]), 0x3C00);
        assert_eq!(half(0.5), 0x3800);
        assert_eq!(half(0.0), 0);
        assert_eq!(pixel_bytes(DXGI_FORMAT_R16G16B16A16_FLOAT), Some(8));
        assert_eq!(pixel_bytes(DXGI_FORMAT_D32_FLOAT), None);
    }
}
