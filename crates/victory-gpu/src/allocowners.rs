//! Diagnostics (`VICTORY_ALLOC_OWNERS=1`; off by default): which module of
//! the process asks for the big blocks of private memory that are none of
//! ours.
//!
//! Why: the memory map (`crate::copystat`) finds, in gameplay on Series S,
//! some 4.5 GB of private memory the layer's own allocator does not count:
//! dozens of 32 MB and 8 MB blocks, single ones of 128 to 256 MB, and more
//! of the process heap than our blocks fill. Whose they are (the graphics
//! driver, the D3D12 runtime, the title) decides whether anything can be
//! done about them. So `VirtualAlloc` and `VirtualFree` are hooked in the
//! import tables of the modules loaded (not ours, not the system's own
//! memory modules), and each commit of 1 MB or more is put down to the
//! module whose code asked for it; a line every 30 seconds says, per
//! module, what it holds now. Beside that, the committed bytes of every
//! heap of the process, the default one (the layer's allocator) marked.
//!
//! What it does not see: memory a module asks the kernel for without
//! `VirtualAlloc` (`NtAllocateVirtualMemory`, `VirtualAlloc2`, sections),
//! and what was allocated before the hooks went in (the device's start).
//! From dexyl7/VIctorious.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};

const MEM_COMMIT: u32 = 0x1000;
const MEM_RELEASE: u32 = 0x8000;
const PAGE_READWRITE: u32 = 0x04;
const FROM_ADDRESS: u32 = 0x4;
const UNCHANGED_REFCOUNT: u32 = 0x2;
/// The least commit counted.
const LEAST: usize = 1 << 20;

#[repr(C)]
#[derive(Default)]
struct HeapSummary {
    cb: u32,
    allocated: usize,
    committed: usize,
    reserved: usize,
    max_reserve: usize,
}

#[link(name = "kernel32", kind = "raw-dylib")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    fn GetModuleHandleExW(flags: u32, name: *const c_void, module: *mut *mut c_void) -> i32;
    fn GetModuleFileNameW(module: *mut c_void, name: *mut u16, size: u32) -> u32;
    fn VirtualProtect(address: *mut c_void, size: usize, protect: u32, old: *mut u32) -> i32;
    fn GetCurrentProcess() -> *mut c_void;
    fn K32EnumProcessModules(process: *mut c_void, modules: *mut *mut c_void, bytes: u32, needed: *mut u32) -> i32;
    fn GetProcessHeaps(count: u32, heaps: *mut *mut c_void) -> u32;
    fn GetProcessHeap() -> *mut c_void;
    fn HeapSummary(heap: *mut c_void, flags: u32, summary: *mut HeapSummary) -> i32;
    fn HeapLock(heap: *mut c_void) -> i32;
    fn HeapUnlock(heap: *mut c_void) -> i32;
    fn HeapWalk(heap: *mut c_void, entry: *mut HeapEntry) -> i32;
    fn RtlCaptureStackBackTrace(skip: u32, count: u32, frames: *mut *mut c_void, hash: *mut u32) -> u16;
}

/// `PROCESS_HEAP_ENTRY` (x64: 40 bytes).
#[repr(C)]
struct HeapEntry {
    data: *mut c_void,
    size: u32,
    overhead: u8,
    region: u8,
    flags: u16,
    rest: [u64; 3],
}

/// The default heap walked whole: bytes of blocks in use (anyone's), bytes
/// free inside committed memory (what is held and not used: the heap's
/// fragmentation), blocks in use. The heap is locked meanwhile (a stall of
/// the process's allocations, so not more often than a minute).
fn walk_default() -> (u64, u64, u64) {
    const BUSY: u16 = 0x4;
    const REGION: u16 = 0x1;
    const UNCOMMITTED: u16 = 0x2;
    let (mut busy, mut free, mut blocks) = (0u64, 0u64, 0u64);
    // SAFETY: the process's own heap, locked while it is walked; nothing is
    // allocated meanwhile on this thread.
    unsafe {
        let heap = GetProcessHeap();
        if HeapLock(heap) == 0 {
            return (0, 0, 0);
        }
        let mut entry = HeapEntry { data: std::ptr::null_mut(), size: 0, overhead: 0, region: 0, flags: 0, rest: [0; 3] };
        while HeapWalk(heap, &mut entry) != 0 {
            if entry.flags & BUSY != 0 {
                busy += u64::from(entry.size) + u64::from(entry.overhead);
                blocks += 1;
            } else if entry.flags & (REGION | UNCOMMITTED) == 0 {
                free += u64::from(entry.size);
            }
        }
        HeapUnlock(heap);
    }
    (busy, free, blocks)
}

type Alloc = unsafe extern "system" fn(*mut c_void, usize, u32, u32) -> *mut c_void;
type Free = unsafe extern "system" fn(*mut c_void, usize, u32) -> i32;

/// The real functions (kernel32's exports, which the import tables named).
static REAL_ALLOC: AtomicUsize = AtomicUsize::new(0);
static REAL_FREE: AtomicUsize = AtomicUsize::new(0);

/// Module names by number; the live blocks by address: (module, bytes).
struct Book {
    names: Vec<String>,
    by_module: HashMap<usize, u16>,
    live: HashMap<usize, (u16, usize)>,
}

static BOOK: Mutex<Option<Book>> = Mutex::new(None);
/// Commits of 1 MB or more seen, and their bytes.
static SEEN: [AtomicU64; 2] = [const { AtomicU64::new(0) }; 2];

thread_local! {
    /// Inside a hook already (the book's own allocations).
    static INSIDE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_ALLOC_OWNERS").is_some_and(|v| v == "1"))
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The module whose code is at `at`, by number.
fn module_of(book: &mut Book, at: *mut c_void) -> u16 {
    let mut module = std::ptr::null_mut();
    // SAFETY: a plain query; no reference is taken.
    let found = unsafe { GetModuleHandleExW(FROM_ADDRESS | UNCHANGED_REFCOUNT, at, &mut module) } != 0;
    let key = if found { module as usize } else { 0 };
    if let Some(&n) = book.by_module.get(&key) {
        return n;
    }
    let name = if found {
        let mut buf = [0u16; 260];
        // SAFETY: room for the name.
        let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        let path = String::from_utf16_lossy(&buf[..len]);
        path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string()
    } else {
        "(no module: generated code or the title)".to_string()
    };
    let n = book.names.len() as u16;
    book.names.push(name);
    book.by_module.insert(key, n);
    n
}

unsafe extern "system" fn hooked_alloc(at: *mut c_void, size: usize, kind: u32, protect: u32) -> *mut c_void {
    // SAFETY: set before any import table points here.
    let real: Alloc = unsafe { std::mem::transmute(REAL_ALLOC.load(Relaxed)) };
    // SAFETY: the caller's call, passed on unchanged.
    let got = unsafe { real(at, size, kind, protect) };
    if got.is_null() || size < LEAST || kind & MEM_COMMIT == 0 || INSIDE.with(|i| i.replace(true)) {
        return got;
    }
    let mut frames = [std::ptr::null_mut(); 1];
    // SAFETY: room for one frame: the caller of this hook.
    let n = unsafe { RtlCaptureStackBackTrace(1, 1, frames.as_mut_ptr(), std::ptr::null_mut()) };
    SEEN[0].fetch_add(1, Relaxed);
    SEEN[1].fetch_add(size as u64, Relaxed);
    {
        let mut book = BOOK.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(book) = book.as_mut() {
            let module = if n == 1 { module_of(book, frames[0]) } else { module_of(book, std::ptr::null_mut()) };
            book.live.insert(got as usize, (module, size));
        }
    }
    INSIDE.with(|i| i.set(false));
    got
}

unsafe extern "system" fn hooked_free(at: *mut c_void, size: usize, kind: u32) -> i32 {
    // SAFETY: set before any import table points here.
    let real: Free = unsafe { std::mem::transmute(REAL_FREE.load(Relaxed)) };
    if kind & MEM_RELEASE != 0 && !INSIDE.with(|i| i.replace(true)) {
        if let Some(book) = BOOK.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            book.live.remove(&(at as usize));
        }
        INSIDE.with(|i| i.set(false));
    }
    // SAFETY: the caller's call, passed on unchanged.
    unsafe { real(at, size, kind) }
}

/// Points the import-table entries of `module` that hold `from` at `to`.
/// Returns how many it changed.
///
/// # Safety
/// `module` is a module mapped in this process.
unsafe fn patch(module: *mut c_void, from: usize, to: usize) -> usize {
    let base = module as usize;
    // SAFETY: a mapped image's headers, as the loader laid them out.
    unsafe {
        let dos = base as *const u8;
        if *dos.cast::<u16>() != 0x5A4D {
            return 0;
        }
        let nt = base + *(dos.add(0x3C).cast::<u32>()) as usize;
        if *(nt as *const u32) != 0x4550 {
            return 0;
        }
        // PE32+: the optional header after the 4-byte signature and the
        // 20-byte file header; the import directory is data directory 1,
        // at 112 + 8 in it.
        let optional = nt + 24;
        if *(optional as *const u16) != 0x20B {
            return 0;
        }
        let dir = optional + 112 + 8;
        let (rva, len) = (*(dir as *const u32) as usize, *((dir + 4) as *const u32) as usize);
        if rva == 0 || len == 0 {
            return 0;
        }
        let mut changed = 0;
        let mut desc = base + rva;
        // IMAGE_IMPORT_DESCRIPTOR: 20 bytes, FirstThunk at +16; a zero one ends.
        while *((desc + 12) as *const u32) != 0 {
            let first = *((desc + 16) as *const u32) as usize;
            if first != 0 {
                let mut slot = (base + first) as *mut usize;
                while *slot != 0 {
                    if *slot == from {
                        let mut old = 0;
                        if VirtualProtect(slot.cast(), 8, PAGE_READWRITE, &mut old) != 0 {
                            *slot = to;
                            VirtualProtect(slot.cast(), 8, old, &mut old);
                            changed += 1;
                        }
                    }
                    slot = slot.add(1);
                }
            }
            desc += 20;
        }
        changed
    }
}

/// Hooks the modules loaded now (and again later: the shader compiler
/// comes in when first used); an entry hooked already holds ours.
pub(crate) fn hook() {
    if !on() {
        return;
    }
    // SAFETY: plain lookups of system exports and module lists; the
    // patches change import-table entries of mapped modules only.
    unsafe {
        let kernel32 = GetModuleHandleW(wide("kernel32.dll").as_ptr());
        let kernelbase = GetModuleHandleW(wide("kernelbase.dll").as_ptr());
        if kernel32.is_null() {
            return;
        }
        let alloc = GetProcAddress(kernel32, c"VirtualAlloc".as_ptr().cast()) as usize;
        let free = GetProcAddress(kernel32, c"VirtualFree".as_ptr().cast()) as usize;
        let based = |name: &std::ffi::CStr| {
            if kernelbase.is_null() { 0 } else { GetProcAddress(kernelbase, name.as_ptr().cast()) as usize }
        };
        let (alloc_base, free_base) = (based(c"VirtualAlloc"), based(c"VirtualFree"));
        if alloc == 0 || free == 0 {
            return;
        }
        let _ = REAL_ALLOC.compare_exchange(0, alloc, Relaxed, Relaxed);
        let _ = REAL_FREE.compare_exchange(0, free, Relaxed, Relaxed);
        BOOK.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(|| Book {
            names: Vec::new(),
            by_module: HashMap::new(),
            live: HashMap::new(),
        });
        let mut modules = vec![std::ptr::null_mut(); 1024];
        let mut needed = 0;
        if K32EnumProcessModules(GetCurrentProcess(), modules.as_mut_ptr(), (modules.len() * 8) as u32, &mut needed) == 0 {
            return;
        }
        modules.truncate((needed as usize / 8).min(modules.len()));
        // Not ours (the executable, first in the list) and not the system's
        // own memory modules.
        let skip = ["kernel32.dll", "kernelbase.dll", "ntdll.dll"];
        let mut changed = 0;
        for &module in modules.iter().skip(1) {
            let mut buf = [0u16; 260];
            let len = GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) as usize;
            let name = String::from_utf16_lossy(&buf[..len]).to_ascii_lowercase();
            if skip.iter().any(|s| name.ends_with(s)) {
                continue;
            }
            for (from, to) in [
                (alloc, hooked_alloc as *const () as usize),
                (alloc_base, hooked_alloc as *const () as usize),
                (free, hooked_free as *const () as usize),
                (free_base, hooked_free as *const () as usize),
            ] {
                if from != 0 {
                    changed += patch(module, from, to);
                }
            }
        }
        if changed > 0 {
            tracing::info!(changed, modules = modules.len(), "VirtualAlloc/VirtualFree hooked in modules' import tables (VICTORY_ALLOC_OWNERS)");
        }
    }
}

/// Every 30 seconds: what each module holds of the commits seen, and the
/// heaps of the process.
pub(crate) fn line() {
    if !on() {
        return;
    }
    static LAST: AtomicU64 = AtomicU64::new(0);
    let now = crate::host::now_ms();
    let last = LAST.load(Relaxed);
    if now < last + 20_000 || LAST.compare_exchange(last, now, Relaxed, Relaxed).is_err() {
        return;
    }
    // Modules loaded since (the shader compiler).
    hook();
    INSIDE.with(|i| i.set(true));
    let held = {
        let book = BOOK.lock().unwrap_or_else(|e| e.into_inner());
        book.as_ref().map(|book| {
            let mut by: Vec<(usize, u64, u64)> = vec![(0, 0, 0); book.names.len()];
            for (i, b) in by.iter_mut().enumerate() {
                b.0 = i;
            }
            for &(module, bytes) in book.live.values() {
                by[module as usize].1 += 1;
                by[module as usize].2 += bytes as u64;
            }
            by.sort_unstable_by(|a, b| b.2.cmp(&a.2));
            by.iter()
                .filter(|b| b.1 > 0)
                .map(|&(i, n, bytes)| format!("{} {} blocks {} MB", book.names[i], n, bytes >> 20))
                .collect::<Vec<_>>()
                .join("; ")
        })
    };
    // SAFETY: plain queries of this process's heaps.
    let heaps = unsafe {
        let mut list = vec![std::ptr::null_mut(); 256];
        let n = (GetProcessHeaps(list.len() as u32, list.as_mut_ptr()) as usize).min(list.len());
        let default = GetProcessHeap();
        let mut sizes: Vec<(u64, bool)> = list[..n]
            .iter()
            .filter_map(|&h| {
                let mut s = HeapSummary { cb: size_of::<HeapSummary>() as u32, ..Default::default() };
                (HeapSummary(h, 0, &mut s) != 0).then_some((s.committed as u64, h == default))
            })
            .collect();
        sizes.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        let all: u64 = sizes.iter().map(|s| s.0).sum();
        let top: Vec<String> = sizes
            .iter()
            .take(8)
            .map(|&(c, d)| format!("{} MB{}", c >> 20, if d { " (the default heap: ours)" } else { "" }))
            .collect();
        format!("{} heaps, {} MB committed in all; the largest: {}", n, all >> 20, top.join(", "))
    };
    // The default heap whole, once a minute: whether what it holds beyond our
    // own blocks is in use by others or free space (fragmentation).
    static WALKED: AtomicU64 = AtomicU64::new(0);
    let walked = WALKED.load(Relaxed);
    let walk = if now >= walked + 60_000 && WALKED.compare_exchange(walked, now, Relaxed, Relaxed).is_ok() {
        let began = std::time::Instant::now();
        let (busy, free, blocks) = walk_default();
        format!(
            "; the default heap walked ({} ms): {} MB in {} blocks in use (anyone's, ours counted apart as 'our own allocations'), {} MB free inside it",
            began.elapsed().as_millis(),
            busy >> 20,
            blocks,
            free >> 20
        )
    } else {
        String::new()
    };
    INSIDE.with(|i| i.set(false));
    tracing::info!(
        "private memory by the module that asked for it (VICTORY_ALLOC_OWNERS; commits of 1 MB or more through VirtualAlloc since the device was made, {} seen, {} MB): {}; heaps: {}{}",
        SEEN[0].load(Relaxed),
        SEEN[1].load(Relaxed) >> 20,
        held.unwrap_or_default(),
        heaps,
        walk
    );
}
