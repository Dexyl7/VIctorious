//! A timeline of a few seconds of play (`VICTORY_TIMELINE=1`): when the
//! title submits and presents, when its threads open and close lists, when
//! we prepare submissions, when the card is done with them, when we land
//! them and when the title's fences are set. Written as a trace file that
//! chrome://tracing or ui.perfetto.dev opens, and summed up in the log.
//!
//! Why: the frame took 80 ms with no thread busy for more than 70 of them
//! and the card idle for half; what each step waits for is not in the
//! totals the frame line gives.
//!
//! `VICTORY_TIMELINE_AFTER` (seconds from the first event, 120) and
//! `VICTORY_TIMELINE_FOR` (seconds recorded, 3) say when;
//! `VICTORY_TIMELINE_OUT` where (`timeline.json` in the working folder).

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

struct Event {
    name: &'static str,
    /// Microseconds from [`base`].
    start: f64,
    /// Microseconds; 0: an instant.
    dur: f64,
    thread: u32,
    arg: u64,
}

static ACTIVE: AtomicBool = AtomicBool::new(false);
static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());
static NAMES: Mutex<Vec<(u32, String)>> = Mutex::new(Vec::new());

fn base() -> Instant {
    static BASE: OnceLock<Instant> = OnceLock::new();
    *BASE.get_or_init(Instant::now)
}

fn seconds(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| *v >= 0.0)
        .unwrap_or(default)
}

/// Whether a timeline was asked for; the first call starts the clock.
fn armed() -> bool {
    static ARMED: OnceLock<bool> = OnceLock::new();
    *ARMED.get_or_init(|| {
        if std::env::var_os("VICTORY_TIMELINE").is_none_or(|v| v == "0") {
            return false;
        }
        let after = seconds("VICTORY_TIMELINE_AFTER", 120.0);
        let lasting = seconds("VICTORY_TIMELINE_FOR", 3.0).clamp(0.1, 30.0);
        let out = std::env::var("VICTORY_TIMELINE_OUT").unwrap_or_else(|_| "timeline.json".into());
        base();
        let spawned = std::thread::Builder::new().name("victory-timeline".into()).spawn(move || {
            std::thread::sleep(Duration::from_secs_f64(after));
            tracing::info!(seconds = lasting, "timeline recording (VICTORY_TIMELINE=1)");
            ACTIVE.store(true, Ordering::SeqCst);
            std::thread::sleep(Duration::from_secs_f64(lasting));
            ACTIVE.store(false, Ordering::SeqCst);
            // Spans begun inside the window end shortly after it.
            std::thread::sleep(Duration::from_millis(300));
            write(&out);
        });
        spawned.is_ok()
    })
}

/// Whether events are being kept now.
#[inline]
pub(crate) fn on() -> bool {
    armed() && ACTIVE.load(Ordering::Relaxed)
}

fn thread() -> u32 {
    #[cfg(windows)]
    // SAFETY: a plain query.
    let id = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
    #[cfg(not(windows))]
    let id = 0;
    thread_local!(static NAMED: Cell<bool> = const { Cell::new(false) });
    if !NAMED.with(Cell::get) {
        NAMED.with(|n| n.set(true));
        let name = std::thread::current().name().map_or_else(|| format!("title thread {id}"), str::to_owned);
        NAMES.lock().unwrap_or_else(|e| e.into_inner()).push((id, name));
    }
    id
}

fn push(name: &'static str, start: Instant, end: Instant, arg: u64) {
    let b = base();
    let at = start.saturating_duration_since(b).as_secs_f64() * 1e6;
    let dur = end.saturating_duration_since(start).as_secs_f64() * 1e6;
    let thread = thread();
    EVENTS.lock().unwrap_or_else(|e| e.into_inner()).push(Event {
        name,
        start: at,
        dur,
        thread,
        arg,
    });
}

/// A span from now until the value is dropped.
pub(crate) struct Span {
    name: &'static str,
    start: Instant,
    arg: u64,
    live: bool,
}

impl Span {
    /// The span's number, known only later (a submission's fence value).
    pub(crate) fn set_arg(&mut self, arg: u64) {
        self.arg = arg;
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if self.live {
            push(self.name, self.start, Instant::now(), self.arg);
        }
    }
}

pub(crate) fn span(name: &'static str, arg: u64) -> Span {
    Span {
        name,
        start: Instant::now(),
        arg,
        live: on(),
    }
}

/// Something that happened now.
pub(crate) fn mark(name: &'static str, arg: u64) {
    if on() {
        let now = Instant::now();
        push(name, now, now, arg);
    }
}

thread_local!(static LIST_OPEN: Cell<Option<Instant>> = const { Cell::new(None) });

/// The title opened (reset) a list on this thread.
pub(crate) fn list_open() {
    if on() {
        LIST_OPEN.with(|o| o.set(Some(Instant::now())));
    }
}

/// The title closed a list on this thread: the span since it was opened.
pub(crate) fn list_closed() {
    if let Some(start) = LIST_OPEN.with(Cell::take)
        && on()
    {
        push("list recorded", start, Instant::now(), 0);
    }
}

fn write(out: &str) {
    use std::fmt::Write as _;
    let events = std::mem::take(&mut *EVENTS.lock().unwrap_or_else(|e| e.into_inner()));
    let names = NAMES.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let mut text = String::with_capacity(events.len() * 110 + 4096);
    text.push_str("{\"traceEvents\":[\n");
    for (id, name) in &names {
        let name: String = name.chars().filter(|c| *c != '"' && *c != '\\').collect();
        let _ = writeln!(
            text,
            "{{\"ph\":\"M\",\"name\":\"thread_name\",\"pid\":1,\"tid\":{id},\"args\":{{\"name\":\"{name}\"}}}},"
        );
    }
    for e in &events {
        if e.dur > 0.0 {
            let _ = writeln!(
                text,
                "{{\"ph\":\"X\",\"name\":\"{}\",\"pid\":1,\"tid\":{},\"ts\":{:.1},\"dur\":{:.1},\"args\":{{\"n\":{}}}}},",
                e.name, e.thread, e.start, e.dur, e.arg
            );
        } else {
            let _ = writeln!(
                text,
                "{{\"ph\":\"i\",\"s\":\"t\",\"name\":\"{}\",\"pid\":1,\"tid\":{},\"ts\":{:.1},\"args\":{{\"n\":{}}}}},",
                e.name, e.thread, e.start, e.arg
            );
        }
    }
    text.push_str("{\"ph\":\"M\",\"name\":\"process_name\",\"pid\":1,\"args\":{\"name\":\"victory\"}}\n]}\n");
    match std::fs::write(out, text) {
        Ok(()) => tracing::info!(events = events.len(), out, "timeline written (VICTORY_TIMELINE=1)"),
        Err(e) => tracing::warn!(error = %e, out, "the timeline could not be written"),
    }
}

/// A measurement, not a fix (`VICTORY_SLOW_TEST=1`): in turn, 20 s each,
/// nothing added; 10 ms of busy work a frame on the submitting thread; 2 us
/// of busy work per draw on the recording threads. The part whose slowing
/// slows the frame by as much is the one the frame waits for.
fn slow_phase() -> u8 {
    static ON: OnceLock<bool> = OnceLock::new();
    if !*ON.get_or_init(|| std::env::var_os("VICTORY_SLOW_TEST").is_some_and(|v| v == "1")) {
        return 0;
    }
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    let phase = ((EPOCH.get_or_init(Instant::now).elapsed().as_secs() / 20) % 3) as u8;
    static LAST: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(9);
    if LAST.swap(phase, Ordering::Relaxed) != phase {
        tracing::info!(
            phase,
            "slow test switched (VICTORY_SLOW_TEST=1): {}",
            ["nothing added", "SUBMIT +10 ms a frame", "RECORD +2 us a draw"][phase as usize]
        );
    }
    phase
}

fn spin(how_long: Duration) {
    let start = Instant::now();
    while start.elapsed() < how_long {
        std::hint::spin_loop();
    }
}

/// At each submission we prepare: 10 ms once a frame in phase 1.
pub(crate) fn slow_test_submit() {
    if slow_phase() != 1 {
        return;
    }
    static FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    let frame = crate::frame::current();
    if FRAME.swap(frame, Ordering::Relaxed) != frame {
        spin(Duration::from_millis(10));
    }
}

/// At each draw's bind: 2 us in phase 2.
pub(crate) fn slow_test_draw() {
    if slow_phase() == 2 {
        spin(Duration::from_micros(2));
    }
}

/// A hunt for what the title sizes its work by (`VICTORY_AUTOPILOT_TEST=1`):
/// in turn, 20 s each, 0 nothing; 1 the landing of the first flight of each
/// frame held 10 ms (fences, bottom-of-pipe writes and their timestamps
/// all reach the title later, no CPU work added); 2 system timestamps run
/// twice as fast. `u8::MAX` without the switch.
pub(crate) fn autopilot_phase() -> u8 {
    static ON: OnceLock<bool> = OnceLock::new();
    if !*ON.get_or_init(|| std::env::var_os("VICTORY_AUTOPILOT_TEST").is_some_and(|v| v == "1")) {
        return u8::MAX;
    }
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    let phase = ((EPOCH.get_or_init(Instant::now).elapsed().as_secs() / 20) % 3) as u8;
    static LAST: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(9);
    if LAST.swap(phase, Ordering::Relaxed) != phase {
        tracing::info!(
            phase,
            "autopilot test switched (VICTORY_AUTOPILOT_TEST=1): {}",
            ["nothing changed", "LANDING held 10 ms a frame", "SYSTEM TIMESTAMPS twice as fast"][phase as usize]
        );
    }
    phase
}

/// In phase 1: the first landing of each frame waits 10 ms.
pub(crate) fn autopilot_hold_landing() {
    if autopilot_phase() != 1 {
        return;
    }
    static FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    let frame = crate::frame::current();
    if FRAME.swap(frame, Ordering::Relaxed) != frame {
        std::thread::sleep(Duration::from_millis(10));
    }
}
