//! The console's audio DSP (`chad.dll`): streams that convolve blocks of
//! float samples with an impulse response (reverb), or run other block
//! transforms, on a coprocessor.
//!
//! A title connects once with a memory block for the DSP, activates
//! streams, and submits one command per block (gains, an input and an
//! output buffer). Activation hands back a status block whose second word is
//! the ticket of the last completed command and whose fourth is its result;
//! the title checks it one block later and treats a late command as an
//! overrun. Commands complete when submitted here.
//!
//! The transforms themselves are not computed: the hardware takes impulse
//! responses already in its own partitioned frequency-domain layout, which
//! is not known. Outputs are silence, so reverb returns are dry.
// shortcut: no convolution or block transform; outputs are zeros. Needs the
// coprocessor's impulse response and spectrum layouts to do better.
#![allow(non_snake_case)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::c_void;
use std::sync::Mutex;

use super::xasync::E_INVALIDARG;
use super::xgameruntime::{HRESULT, S_OK};
use crate::{HleModule, hle_export};

const E_POINTER: HRESULT = 0x8000_4003_u32 as i32;
const E_OUTOFMEMORY: HRESULT = 0x8007_000E_u32 as i32;

const CONVOLUTION: u32 = 1;
/// Option bits that need a stereo stream, and one convolution refuses.
const STEREO_OPTIONS: u32 = 2 | 8;
const NOT_FOR_CONVOLUTION: u32 = 4;

pub struct Dsp {
    max_streams: u32,
    active: Mutex<u32>,
}

/// What activation describes.
#[repr(C, packed)]
pub struct StreamDesc {
    kind: u32,
    options: u32,
    frames: u32,
    channels: u32,
    ir_floats: u32,
    ir: *const f32,
}

/// The status block a stream's owner polls.
#[repr(C)]
pub struct Status {
    submitted: u32,
    completed: u32,
    reserved: u32,
    result: HRESULT,
}

pub struct Stream {
    dsp: *mut Dsp,
    frames: u32,
    channels: u32,
    status: Box<Status>,
    /// What it was activated as (for the log and [`pass`]).
    kind: u32,
    options: u32,
    ir_floats: u32,
    number: u32,
    /// Since its last log line: blocks submitted, and the sum of squares of
    /// the first channel's input samples.
    blocks: u64,
    energy: f64,
    samples: u64,
    /// The phone filter's state ([`Phone`]), for a short impulse response.
    phone: Phone,
    /// The convolution itself, made from the impulse response the title
    /// handed over ([`Convolver`]); none if it could not be read.
    convolver: Option<Box<Convolver>>,
}

/// Block length and transform length of the coprocessor's convolution.
const BLOCK: usize = 1024;
const N: usize = 2 * BLOCK;

/// A complex number.
#[derive(Clone, Copy, Default)]
struct C(f32, f32);

impl C {
    fn mul(self, o: C) -> C {
        C(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
}

/// An in-place radix-2 transform of `N` points (`inverse`: the conjugate
/// one, scaled by 1/N).
fn fft(x: &mut [C; N], inverse: bool) {
    static TWIDDLES: std::sync::OnceLock<Vec<C>> = std::sync::OnceLock::new();
    let tw = TWIDDLES.get_or_init(|| {
        (0..N / 2)
            .map(|k| {
                let a = -2.0 * std::f64::consts::PI * k as f64 / N as f64;
                C(a.cos() as f32, a.sin() as f32)
            })
            .collect()
    });
    let bits = N.trailing_zeros();
    for i in 0..N {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            x.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= N {
        let step = N / len;
        for start in (0..N).step_by(len) {
            for k in 0..len / 2 {
                let w = tw[k * step];
                let w = if inverse { C(w.0, -w.1) } else { w };
                let (a, b) = (x[start + k], x[start + k + len / 2].mul(w));
                x[start + k] = C(a.0 + b.0, a.1 + b.1);
                x[start + k + len / 2] = C(a.0 - b.0, a.1 - b.1);
            }
        }
        len *= 2;
    }
    if inverse {
        let scale = 1.0 / N as f32;
        for v in x.iter_mut() {
            *v = C(v.0 * scale, v.1 * scale);
        }
    }
}

/// A uniformly partitioned convolution (overlap-save, blocks of [`BLOCK`]),
/// from an impulse response in the coprocessor's layout. That layout, read
/// from the title's own (2026-11-28): partitions of 2,048 floats, each the
/// first 1,024 bins (real, imaginary, in turn) of the transform of 2,048
/// samples of the response, the partitions' windows 1,024 samples apart and
/// the first one beginning 1,024 samples before the response; so the second
/// half of each window is the next 1,024 samples of the response. A phone's
/// read back as 300-4,000 Hz with -40 dB above 6 kHz; the rooms' as
/// decaying reverbs of 1.3-2 s.
struct Convolver {
    /// Per partition, the spectrum (bins 0..=BLOCK) of its samples padded.
    parts: Vec<Vec<C>>,
    /// The spectra of the last inputs, newest at `at`.
    inputs: Vec<Vec<C>>,
    at: usize,
    last: Vec<f32>,
    /// Blocks in a row with no input: past `parts.len()` the output is zero
    /// and nothing is computed.
    quiet: usize,
}

impl Convolver {
    fn new(ir: &[f32]) -> Option<Box<Convolver>> {
        if ir.len() < N || ir.len() % N != 0 {
            return None;
        }
        let mut response: Vec<f32> = Vec::with_capacity(ir.len() / 2);
        for part in ir.chunks_exact(N) {
            let mut x = [C::default(); N];
            for k in 0..BLOCK {
                let (re, im) = (part[2 * k], part[2 * k + 1]);
                let c = if re.is_finite() && im.is_finite() { C(re, im) } else { C::default() };
                x[k] = c;
                if k > 0 {
                    x[N - k] = C(c.0, -c.1);
                }
            }
            fft(&mut x, true);
            response.extend(x[BLOCK..].iter().map(|v| v.0));
        }
        // What is left of it past the last sample that counts.
        let peak = response.iter().fold(0f32, |m, v| m.max(v.abs()));
        if peak == 0.0 || !peak.is_finite() {
            return None;
        }
        let end = response.iter().rposition(|v| v.abs() > peak * 1e-4).map_or(0, |i| i + 1);
        let parts: Vec<Vec<C>> = response[..end]
            .chunks(BLOCK)
            .map(|h| {
                let mut x = [C::default(); N];
                for (i, &v) in h.iter().enumerate() {
                    x[i] = C(v, 0.0);
                }
                fft(&mut x, false);
                x[..=BLOCK].to_vec()
            })
            .collect();
        let n = parts.len();
        Some(Box::new(Convolver { parts, inputs: vec![vec![C::default(); BLOCK + 1]; n], at: 0, last: vec![0.0; BLOCK], quiet: usize::MAX / 2 }))
    }

    /// One block in, one out.
    fn block(&mut self, input: &[f32], output: &mut [f32]) {
        let silent = input.iter().all(|v| v.abs() < 1e-9);
        self.quiet = if silent { self.quiet.saturating_add(1) } else { 0 };
        if self.quiet > self.parts.len() {
            if self.quiet == self.parts.len() + 1 {
                for s in &mut self.inputs {
                    s.fill(C::default());
                }
                self.last.fill(0.0);
            }
            output.fill(0.0);
            return;
        }
        let mut x = [C::default(); N];
        for i in 0..BLOCK {
            x[i] = C(self.last[i], 0.0);
            x[BLOCK + i] = C(input.get(i).copied().unwrap_or(0.0), 0.0);
        }
        self.last.copy_from_slice(&input[..BLOCK.min(input.len())]);
        fft(&mut x, false);
        let n = self.parts.len();
        self.at = (self.at + 1) % n;
        self.inputs[self.at].copy_from_slice(&x[..=BLOCK]);
        let mut y = [C::default(); N];
        for (k, part) in self.parts.iter().enumerate() {
            let spectrum = &self.inputs[(self.at + n - k) % n];
            for b in 0..=BLOCK {
                let p = spectrum[b].mul(part[b]);
                y[b] = C(y[b].0 + p.0, y[b].1 + p.1);
            }
        }
        for b in 1..BLOCK {
            y[N - b] = C(y[b].0, -y[b].1);
        }
        fft(&mut y, true);
        for (o, v) in output.iter_mut().zip(&y[BLOCK..]) {
            *o = v.0;
        }
    }
}

/// Whether convolutions are carried out from the impulse responses the
/// title hands over ([`Convolver`]): `VICTORY_DSP_CONVOLVE=0` none (the
/// phone band of [`Phone`] for short ones), `phone` only the short ones,
/// unset or `1` all of them, reverbs included.
fn convolve(short: bool) -> bool {
    static MODE: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
    match *MODE.get_or_init(|| match std::env::var("VICTORY_DSP_CONVOLVE").as_deref() {
        Ok("0") => 0,
        Ok("phone") => 1,
        _ => 2,
    }) {
        0 => false,
        1 => short,
        _ => true,
    }
}

/// A convolution whose impulse response has at most this many floats is
/// taken for a voice effect (a phone's or a radio's speaker), not a room's
/// reverb: in the title's phone call its voice streams had 40,960 (less
/// than half a second) and carried the caller's voice, the reverbs
/// 135,168-204,800 with only quiet sends (2026-11-28).
const SHORT_IR: u32 = 65_536;

/// A phone's sound in place of the convolution that is not known: a
/// band-pass of 300-3,400 Hz (a high-pass and a low-pass, second order
/// each, at 48 kHz), what a phone line carries.
#[derive(Default)]
struct Phone {
    high: [f32; 4],
    low: [f32; 4],
}

impl Phone {
    /// Biquad coefficients (b0, b1, b2, a1, a2) of a Butterworth high- or
    /// low-pass at `hz`.
    fn coefficients(hz: f32, high: bool) -> [f32; 5] {
        let w = 2.0 * std::f32::consts::PI * hz / 48_000.0;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
        let a0 = 1.0 + alpha;
        let (b0, b1, b2) = if high {
            ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0)
        } else {
            ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0)
        };
        [b0 / a0, b1 / a0, b2 / a0, -2.0 * cos / a0, (1.0 - alpha) / a0]
    }

    fn step(c: &[f32; 5], z: &mut [f32; 4], x: f32) -> f32 {
        // z: x1, x2, y1, y2.
        let y = c[0] * x + c[1] * z[0] + c[2] * z[1] - c[3] * z[2] - c[4] * z[3];
        *z = [x, z[0], y, z[2]];
        y
    }

    fn sample(&mut self, x: f32) -> f32 {
        static C: std::sync::OnceLock<([f32; 5], [f32; 5])> = std::sync::OnceLock::new();
        let (high, low) = C.get_or_init(|| (Self::coefficients(300.0, true), Self::coefficients(3_400.0, false)));
        let y = Self::step(high, &mut self.high, x);
        Self::step(low, &mut self.low, y)
    }
}

/// Whether every convolution stream's output is its input (dry) instead of
/// silence (`VICTORY_DSP_PASS`): `1` always, `ab` every other 10 seconds
/// (a line says which); unset or `0`: only the voice effects' streams give
/// something ([`phone`]), reverbs are silent as before. Voices the title
/// runs through a convolution to sound like a phone or a radio, and not
/// only a reverb's return, come out of these streams: silent, the other
/// end of a phone call was not heard while the player's lines were.
/// shortcut: dry, not convolved (the impulse response's layout is not
/// known); a reverb return is the dry sound again.
/// Whether a convolution with a short impulse response ([`SHORT_IR`]) gives
/// its input through a phone's band ([`Phone`]) instead of silence
/// (`VICTORY_DSP_PHONE=0`: silence, as before).
fn phone() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_DSP_PHONE").is_none_or(|v| v != "0"))
}

fn pass() -> bool {
    static MODE: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
    let mode = *MODE.get_or_init(|| match std::env::var("VICTORY_DSP_PASS").as_deref() {
        Ok("1") => 1,
        Ok("ab") => 2,
        _ => 0,
    });
    match mode {
        1 => true,
        2 => {
            static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
            static SAID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
            let window = EPOCH.get_or_init(std::time::Instant::now).elapsed().as_secs() / 10;
            let on = window % 2 == 1;
            if SAID.swap(window, std::sync::atomic::Ordering::Relaxed) != window {
                tracing::info!(on, "audio DSP: convolution outputs are their input (VICTORY_DSP_PASS=ab)");
            }
            on
        }
        _ => false,
    }
}

/// One block of work.
#[repr(C)]
pub struct Command {
    gains: [f32; 2],
    input: *const f32,
    output: *mut f32,
    reserved: u64,
}

pub extern "system" fn XDspConnectWithMaximumStreamLimit(
    base: *mut c_void,
    base_length: u32,
    ir_seconds: u32,
    max_streams: u32,
    max_channels: u32,
    out: *mut *mut Dsp,
) -> HRESULT {
    if base.is_null()
        || !(0x1_0000..=0x1F40_0000).contains(&base_length)
        || ir_seconds > 0x80
        || max_streams > 0x100
        || max_channels > 2
    {
        return E_INVALIDARG;
    }
    if out.is_null() {
        return E_POINTER;
    }
    tracing::info!(
        base_length,
        ir_seconds,
        max_streams,
        max_channels,
        "audio DSP connected"
    );
    let dsp = Box::new(Dsp {
        max_streams,
        active: Mutex::new(0),
    });
    // SAFETY: checked non-null.
    unsafe { out.write(Box::into_raw(dsp)) };
    S_OK
}

pub extern "system" fn XDspDisconnect(dsp: *mut Dsp) -> HRESULT {
    if dsp.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: made by `XDspConnectWithMaximumStreamLimit`; disconnected
    // once, after its streams.
    drop(unsafe { Box::from_raw(dsp) });
    S_OK
}

fn valid(desc: &StreamDesc) -> bool {
    let (kind, options, frames, channels) = (desc.kind, desc.options, desc.frames, desc.channels);
    let shape = if kind == CONVOLUTION {
        (1..=2).contains(&channels)
            && matches!(frames, 0x200 | 0x400)
            && options & NOT_FOR_CONVOLUTION == 0
    } else {
        channels == 1 && matches!(frames, 0x200 | 0x400 | 0x800)
    };
    shape && (options & STEREO_OPTIONS == 0 || channels == 2)
}

pub extern "system" fn XDspActivate(
    dsp: *mut Dsp,
    desc: *const StreamDesc,
    status: *mut *const Status,
    stream: *mut *mut Stream,
) -> HRESULT {
    if status.is_null() || stream.is_null() {
        return E_POINTER;
    }
    if dsp.is_null() || desc.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: checked non-null; the title passes its description.
    let desc = unsafe { &*desc };
    if !valid(desc) {
        return E_INVALIDARG;
    }
    let (kind, options, frames, channels, ir_floats) = (desc.kind, desc.options, desc.frames, desc.channels, desc.ir_floats);
    static NUMBER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let number = NUMBER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    tracing::info!(
        number,
        kind,
        options,
        frames,
        channels,
        ir_floats,
        "audio DSP stream activated"
    );
    dump(number, desc);
    {
        // SAFETY: a connection from `XDspConnectWithMaximumStreamLimit`.
        let dsp = unsafe { &*dsp };
        let mut active = dsp.active.lock().unwrap_or_else(|e| e.into_inner());
        if *active >= dsp.max_streams {
            return E_OUTOFMEMORY;
        }
        *active += 1;
    }
    let ir_at = desc.ir;
    let new = Box::new(Stream {
        dsp,
        frames: desc.frames,
        channels: desc.channels,
        status: Box::new(Status {
            submitted: 0,
            completed: 0,
            reserved: 0,
            result: S_OK,
        }),
        kind,
        options,
        ir_floats,
        number,
        blocks: 0,
        energy: 0.0,
        samples: 0,
        phone: Phone::default(),
        convolver: (kind == CONVOLUTION && channels == 1 && !ir_at.is_null() && convolve(ir_floats <= SHORT_IR))
            .then(|| {
                // SAFETY: the title passes `ir_floats` floats at `ir`.
                let ir = unsafe { std::slice::from_raw_parts(ir_at, ir_floats as usize) };
                Convolver::new(ir)
            })
            .flatten(),
    });
    tracing::info!(number, convolved = new.convolver.is_some(), partitions = new.convolver.as_ref().map_or(0, |c| c.parts.len()), "audio DSP stream: convolution");
    // SAFETY: checked non-null.
    unsafe {
        status.write(&*new.status);
        stream.write(Box::into_raw(new));
    }
    S_OK
}

/// Saves a stream's impulse response, as the title hands it over, to
/// `VICTORY_DSP_DUMP=<dir>` (the first 40 streams): `dsp-<number>-kind<k>-
/// <frames>x<channels>-opt<o>-<floats>.f32`, raw little-endian floats. For
/// working out the layout the coprocessor takes them in.
fn dump(number: u32, desc: &StreamDesc) {
    static DIR: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    let Some(dir) = DIR.get_or_init(|| std::env::var_os("VICTORY_DSP_DUMP").map(std::path::PathBuf::from)) else {
        return;
    };
    let (kind, options, frames, channels, floats, ir) = (desc.kind, desc.options, desc.frames, desc.channels, desc.ir_floats, desc.ir);
    if number >= 40 || ir.is_null() || floats == 0 {
        return;
    }
    // SAFETY: the title passes `ir_floats` floats at `ir`.
    let data = unsafe { std::slice::from_raw_parts(ir.cast::<u8>(), floats as usize * 4) };
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join(format!("dsp-{number}-kind{kind}-{frames}x{channels}-opt{options}-{floats}.f32"));
    match std::fs::write(&path, data) {
        Ok(()) => tracing::info!(path = %path.display(), "audio DSP: impulse response saved"),
        Err(e) => tracing::warn!(error = %e, "audio DSP: impulse response not saved"),
    }
}

pub extern "system" fn XDspDeactivate(stream: *mut Stream) -> HRESULT {
    if stream.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: made by `XDspActivate`; deactivated once.
    let stream = unsafe { Box::from_raw(stream) };
    // SAFETY: the connection outlives its streams.
    let dsp = unsafe { &*stream.dsp };
    *dsp.active.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
    S_OK
}

/// Completes the command at once: its output block is silence.
pub extern "system" fn XDspSubmitCommand(
    stream: *mut Stream,
    command: *const Command,
    ticket: *mut u32,
) -> HRESULT {
    if stream.is_null() || command.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: a stream from `XDspActivate`, the title's command.
    let (stream, command) = unsafe { (&mut *stream, &*command) };
    let frames = stream.frames as usize;
    let channels = stream.channels as usize;
    // The first channel's input, if any: a block of `frames` samples at
    // least (read as mono; a stereo input's first half).
    // SAFETY: the title's input buffer holds one block of the stream.
    let input = (!command.input.is_null()).then(|| unsafe { std::slice::from_raw_parts(command.input, frames) });
    if !command.output.is_null() {
        // SAFETY: the title's output buffer holds one block of the stream.
        let output = unsafe { std::slice::from_raw_parts_mut(command.output, frames * channels) };
        match input {
            Some(input) if stream.kind == CONVOLUTION && pass() => {
                for (f, &sample) in input.iter().enumerate() {
                    for c in 0..channels {
                        output[f * channels + c] = sample;
                    }
                }
            }
            Some(input) if stream.convolver.is_some() && frames == BLOCK && !pass() => {
                let gain = command.gains[0] * command.gains[1];
                let mut y = [0f32; BLOCK];
                if let Some(c) = stream.convolver.as_mut() {
                    c.block(input, &mut y);
                }
                for (f, &v) in y.iter().enumerate() {
                    for c in 0..channels {
                        output[f * channels + c] = v * gain;
                    }
                }
            }
            Some(input) if stream.kind == CONVOLUTION && stream.ir_floats <= SHORT_IR && phone() => {
                for (f, &sample) in input.iter().enumerate() {
                    let y = stream.phone.sample(sample);
                    for c in 0..channels {
                        output[f * channels + c] = y;
                    }
                }
            }
            _ => output.fill(0.0),
        }
    }
    if let Some(input) = input {
        stream.blocks += 1;
        stream.samples += input.len() as u64;
        stream.energy += input.iter().map(|&x| f64::from(x) * f64::from(x)).sum::<f64>();
        if stream.blocks % 200 == 0 {
            let rms = (stream.energy / stream.samples.max(1) as f64).sqrt();
            tracing::info!(
                stream = stream.number,
                kind = stream.kind,
                options = stream.options,
                channels = stream.channels,
                ir_floats = stream.ir_floats,
                gains = ?command.gains,
                rms = format_args!("{rms:.5}"),
                "audio DSP stream: 200 blocks in, the input's loudness"
            );
            stream.energy = 0.0;
            stream.samples = 0;
        }
    }
    let status = &mut *stream.status;
    let next = status.submitted.wrapping_add(1);
    // SAFETY: the status block is read by title code on other threads.
    unsafe {
        std::ptr::write_volatile(&mut status.submitted, next);
        std::ptr::write_volatile(&mut status.result, S_OK);
        std::ptr::write_volatile(&mut status.completed, next);
    }
    if !ticket.is_null() {
        // SAFETY: optional out parameter.
        unsafe { ticket.write(next) };
    }
    S_OK
}

pub(crate) fn module() -> HleModule {
    HleModule {
        name: "dsp",
        dlls: &["chad.dll"],
        exports: vec![
            hle_export!(XDspConnectWithMaximumStreamLimit),
            hle_export!(XDspDisconnect),
            hle_export!(XDspActivate),
            hle_export!(XDspDeactivate),
            hle_export!(XDspSubmitCommand),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convolution_stream_as_the_title_drives_it() {
        let mut base = vec![0u8; 0x10_0000];
        let mut dsp = std::ptr::null_mut();
        assert_eq!(
            XDspConnectWithMaximumStreamLimit(
                base.as_mut_ptr().cast(),
                0x10_0000,
                24,
                0x80,
                1,
                &mut dsp
            ),
            S_OK
        );
        let ir = vec![0f32; 0x2_0000];
        // Stereo convolution with option 2, as the title asks for.
        let desc = StreamDesc {
            kind: CONVOLUTION,
            options: 2,
            frames: 0x400,
            channels: 2,
            // A reverb's length: silent ([`SHORT_IR`]; the IR is not read).
            ir_floats: 0x2_0000,
            ir: ir.as_ptr(),
        };
        let (mut status, mut stream) = (std::ptr::null(), std::ptr::null_mut());
        assert_eq!(XDspActivate(dsp, &desc, &mut status, &mut stream), S_OK);
        let mut output = vec![1f32; 0x800];
        let input = vec![0.5f32; 0x800];
        let command = Command {
            gains: [1.0, 1.0],
            input: input.as_ptr(),
            output: output.as_mut_ptr(),
            reserved: 0,
        };
        let mut ticket = 0;
        assert_eq!(XDspSubmitCommand(stream, &command, &mut ticket), S_OK);
        // SAFETY: the stream's status block.
        let (completed, result) = unsafe { ((*status).completed, (*status).result) };
        assert_eq!((completed, result), (ticket, S_OK));
        assert!(output.iter().all(|&s| s == 0.0));
        assert_eq!(XDspDeactivate(stream), S_OK);

        // Mono streams refuse stereo options; transforms are mono only.
        let bad = StreamDesc {
            channels: 1,
            ..desc
        };
        assert_eq!(
            XDspActivate(dsp, &bad, &mut status, &mut stream),
            E_INVALIDARG
        );
        let transform = StreamDesc {
            kind: 2,
            options: 0,
            channels: 1,
            ir_floats: 0,
            ir: std::ptr::null(),
            ..desc
        };
        assert_eq!(
            XDspActivate(dsp, &transform, &mut status, &mut stream),
            S_OK
        );
        assert_eq!(XDspDeactivate(stream), S_OK);
        assert_eq!(XDspDisconnect(dsp), S_OK);
    }
}
