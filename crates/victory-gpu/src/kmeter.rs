//! Diagnostics: the kernel calls of title memory tracking, timed by kind and
//! by the thread that made them (the submitting thread, the helper threads
//! that take work cut in pieces, every other thread: the title's mostly), and
//! the write faults on guarded memory counted the same way. What it is for:
//! the submitting thread spends about 110 of its 280 ms in the kernel in a
//! long frame; this says how much of that is the tracking of title memory
//! (asking the write watch, changing protections, asking what pages are,
//! reading and writing through the kernel) and how many faults the guards
//! cost the title's own threads. The time inside a fault handler is measured;
//! the kernel's dispatch of the exception around it is not (it cannot be from
//! inside), so faults are given as a count too.

#[cfg(feature = "diagnostics")]
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// What a timed call was.
#[derive(Clone, Copy)]
pub(crate) enum K {
    /// `GetWriteWatch` (with its reset).
    Watch = 0,
    /// `VirtualProtect`.
    Protect,
    /// `VirtualQuery`.
    Query,
    /// `ReadProcessMemory`.
    Read,
    /// `NtWriteVirtualMemory` / `WriteProcessMemory`.
    Write,
    /// A write fault on a guarded block, in our handler.
    #[cfg_attr(not(feature = "diagnostics"), allow(dead_code))]
    GuardFault,
}
#[cfg(feature = "diagnostics")]
const KINDS: usize = 6;
#[cfg(feature = "diagnostics")]
const NAMES: [&str; KINDS] = ["write watch", "protections", "page queries", "kernel reads", "kernel writes", "guard faults (in our handler)"];

#[cfg(feature = "diagnostics")]
thread_local! {
    /// 0 every other thread, 1 the submitting thread, 2 a helper thread.
    static ROLE: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

/// This thread prepares submissions.
pub(crate) fn submitting_thread() {
    #[cfg(feature = "diagnostics")]
    ROLE.with(|r| r.set(1));
}

/// This thread takes work cut in pieces.
pub(crate) fn helper_thread() {
    #[cfg(feature = "diagnostics")]
    ROLE.with(|r| r.set(2));
}

#[cfg(feature = "diagnostics")]
static CALLS: [[AtomicU64; KINDS]; 3] = [const { [const { AtomicU64::new(0) }; KINDS] }; 3];
#[cfg(feature = "diagnostics")]
static NS: [[AtomicU64; KINDS]; 3] = [const { [const { AtomicU64::new(0) }; KINDS] }; 3];

/// Runs `f`, a kernel call of kind `k`, timed (diagnostics builds).
#[inline]
pub(crate) fn timed<R>(k: K, f: impl FnOnce() -> R) -> R {
    #[cfg(feature = "diagnostics")]
    let started = std::time::Instant::now();
    let r = f();
    #[cfg(feature = "diagnostics")]
    note(k, started.elapsed().as_nanos() as u64);
    #[cfg(not(feature = "diagnostics"))]
    let _ = k;
    r
}

/// Counts one call of kind `k` that took `ns` on this thread.
#[cfg(feature = "diagnostics")]
pub(crate) fn note(k: K, ns: u64) {
    let role = ROLE.with(|r| r.get()) as usize;
    CALLS[role][k as usize].fetch_add(1, Relaxed);
    NS[role][k as usize].fetch_add(ns, Relaxed);
}

/// With the frame line: the calls per frame over `frames`, by thread.
#[cfg(feature = "diagnostics")]
pub(crate) fn line(frames: f64) {
    if frames <= 0.0 {
        return;
    }
    let mut parts = Vec::new();
    for (role, name) in [(1, "submitting thread"), (2, "helper threads"), (0, "other threads (the title's)")] {
        let mut kinds = Vec::new();
        let mut total = 0.0;
        for k in 0..KINDS {
            let n = CALLS[role][k].swap(0, Relaxed) as f64 / frames;
            let ms = NS[role][k].swap(0, Relaxed) as f64 / 1e6 / frames;
            total += ms;
            if n >= 0.05 {
                kinds.push(format!("{} {:.0} = {:.2} ms", NAMES[k], n, ms));
            }
        }
        parts.push(format!("{name}: {total:.2} ms ({})", kinds.join(", ")));
    }
    tracing::info!("kernel calls of memory tracking per frame (VICTORY kmeter): {}", parts.join("; "));
}
