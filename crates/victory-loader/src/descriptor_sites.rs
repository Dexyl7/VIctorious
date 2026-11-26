//! Renderer port #2, from avatarmanz/VIctory-7900XTX (commits 9997d255, 4cb90bee) (`VICTORY_PORT_DESCRIPTORS=1`, read in `victory-gpu`'s
//! `port_descriptors`): the title's own code that copies descriptor records
//! into its shader-visible heap as bytes tells the layer which slots it wrote.
//!
//! What the title does (the title's executable): each draw's and each
//! dispatch's descriptor tables are allocated from its frame allocator
//! (image +0x4d9f400; the allocator's memory is the placed shader-visible
//! view heap) and filled by two functions, +0x4d9efd0 (graphics) and
//! +0x4eec8e0 (compute), three loops each, one record at a time:
//! `and r, 7FE0h` (the slot's byte offset in the table), `vmovups ymm0,
//! [record]`, `vmovups [table + r], ymm0`. Found with `dumpbin /disasm` from
//! the write faults of run faults-6 (book records "the instruction at a made
//! the fault"); a search of all of `.text` for the same instruction shape
//! finds these six stores and no other.
//!
//! Each store becomes a jump to code of ours that does the store, then, if
//! the address lies in the heap `victory-gpu` names in [`Marks`], sets the
//! slot's byte in a map of slots and its 4 KiB chunk's byte in a map of
//! chunks, and jumps back. The flags are clobbered: at every site the next
//! instructions are a `movzx` and a `cmp`, which sets them before any use
//! (checked as part of the original bytes). Every register is put back.
//!
//! Each site's original bytes (from the `and` to the `cmp`) are compared
//! before anything is written; a site whose bytes differ is left alone and
//! said so (another build of the title).

/// The cells the stubs read, `#[repr(C)]` in `victory-gpu`: the heap's first
/// byte, its length in bytes (0: none, nothing is marked), the map of slots
/// (a byte each), the map of chunks (a byte for 128 slots).
pub const MARKS_LEN: usize = 32;

/// A store of the title's to hook: the window's offset in the image, the
/// store's offset in the window and its length, and the window's bytes.
pub struct Site {
    pub window: usize,
    pub store: usize,
    pub len: usize,
    pub bytes: &'static [u8],
}

/// The six record stores of the table fillers (the title's executable
/// 1.0.2.1, image offsets).
pub const SITES: [Site; 6] = [
    // +0x4d9efd0, loop 1: [rcx+rsi]
    Site { window: 0x4d9f04f, store: 10, len: 5, bytes: &[0x81, 0xe1, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x00, 0xc5, 0xfc, 0x11, 0x04, 0x31, 0x0f, 0xb6, 0x47, 0x0b, 0x39, 0xc5] },
    // loop 2: [rcx+rsi]
    Site { window: 0x4d9f10d, store: 10, len: 5, bytes: &[0x81, 0xe1, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x00, 0xc5, 0xfc, 0x11, 0x04, 0x31, 0x0f, 0xb6, 0x47, 0x0f, 0x41, 0x39, 0xc5] },
    // loop 3: [r9+rsi]
    Site { window: 0x4d9f19c, store: 12, len: 6, bytes: &[0x41, 0x81, 0xe1, 0xe0, 0x7f, 0x00, 0x00, 0xc4, 0xc1, 0x7c, 0x10, 0x02, 0xc4, 0xc1, 0x7c, 0x11, 0x04, 0x31, 0x44, 0x0f, 0xb6, 0x4f, 0x0d, 0x44, 0x39, 0xc8] },
    // +0x4eec8e0, loop 1: [r9+r8]
    Site { window: 0x4eec95d, store: 11, len: 6, bytes: &[0x41, 0x81, 0xe1, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x00, 0xc4, 0x81, 0x7c, 0x11, 0x04, 0x01, 0x0f, 0xb6, 0x42, 0x0b, 0x39, 0xc5] },
    // loop 2: [rdi+r8]
    Site { window: 0x4eeca08, store: 10, len: 6, bytes: &[0x81, 0xe7, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x06, 0xc4, 0xa1, 0x7c, 0x11, 0x04, 0x07, 0x0f, 0xb6, 0x72, 0x0d, 0x39, 0xf0] },
    // loop 3: [r10+r8]
    Site { window: 0x4eeca82, store: 11, len: 6, bytes: &[0x41, 0x81, 0xe2, 0xe0, 0x7f, 0x00, 0x00, 0xc5, 0xfc, 0x10, 0x00, 0xc4, 0x81, 0x7c, 0x11, 0x04, 0x02, 0x0f, 0xb6, 0x42, 0x0f, 0x41, 0x39, 0xc6] },
];

/// `lea rax, [base+index]` for a `vmovups [base+index], ymm0` (VEX, ModRM
/// 04h with a SIB byte, no displacement, the stack pointer not its base):
/// `None` for another form.
fn lea_of(store: &[u8]) -> Option<[u8; 4]> {
    let (x, b, op_at) = match *store.first()? {
        0xc5 => (false, false, 2),
        0xc4 => {
            let p = *store.get(1)?;
            if p & 0x1f != 1 {
                return None;
            }
            (p & 0x40 == 0, p & 0x20 == 0, 3)
        }
        _ => return None,
    };
    let (op, modrm, sib) = (*store.get(op_at)?, *store.get(op_at + 1)?, *store.get(op_at + 2)?);
    if op != 0x11 || modrm != 0x04 || store.len() != op_at + 3 {
        return None;
    }
    // No base (mode 0, base 101), or the stack pointer as base, or rsp as index (none).
    if sib & 7 == 5 || (sib & 7 == 4 && !b) || (sib >> 3 & 7 == 4 && !x) {
        return None;
    }
    Some([0x48 | u8::from(x) << 1 | u8::from(b), 0x8d, 0x04, sib])
}

/// `jmp rel32` at `at` to `to`, if it reaches (as `console_insn`'s).
fn jump(at: usize, to: usize) -> Option<[u8; 5]> {
    let rel = i32::try_from(to as i64 - (at as i64 + 5)).ok()?;
    let [a, b, c, d] = rel.to_le_bytes();
    Some([0xe9, a, b, c, d])
}

/// Room for the stubs, near the title's code.
const STUBS_LEN: usize = 64 << 10;

/// Memory for code within reach of a `jmp rel32` at `at` (as `console_insn`'s).
fn near(at: usize) -> Option<usize> {
    use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc};
    let at = at & !0xffff;
    (0x10_0000..0x7000_0000).step_by(0x10_0000).flat_map(|d| [at.checked_sub(d), at.checked_add(d)]).flatten().find_map(|place| {
        // SAFETY: reserves free memory at that address or fails.
        let got = unsafe { VirtualAlloc(place as *const _, STUBS_LEN, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) } as usize;
        (got != 0).then_some(got)
    })
}

/// The code at `to` for the store `store` at `at`: the store, the marks, back
/// to `at + store.len()`.
fn stub(store: &[u8], at: usize, to: usize, marks: usize) -> Option<Vec<u8>> {
    let lea = lea_of(store)?;
    let mut out = store.to_vec();
    out.push(0x50); // push rax
    out.extend(lea); // lea rax, [base+index]
    out.extend([0x52, 0x51]); // push rdx ; push rcx
    out.extend([0x48, 0xba]); // mov rdx, marks
    out.extend((marks as u64).to_le_bytes());
    out.extend([0x48, 0x2b, 0x02]); // sub rax, [rdx]
    out.extend([0x48, 0x3b, 0x42, 0x08]); // cmp rax, [rdx+8]
    out.extend([0x73, 0x18]); // jae past the marks
    out.extend([0x48, 0xc1, 0xe8, 0x05]); // shr rax, 5: the slot
    out.extend([0x48, 0x8b, 0x4a, 0x10]); // mov rcx, [rdx+16]
    out.extend([0xc6, 0x04, 0x01, 0x01]); // mov byte [rcx+rax], 1
    out.extend([0x48, 0xc1, 0xe8, 0x07]); // shr rax, 7: the chunk
    out.extend([0x48, 0x8b, 0x4a, 0x18]); // mov rcx, [rdx+24]
    out.extend([0xc6, 0x04, 0x01, 0x01]); // mov byte [rcx+rax], 1
    out.extend([0x59, 0x5a, 0x58]); // pop rcx ; pop rdx ; pop rax
    let back = jump(to + out.len(), at + store.len())?;
    out.extend(back);
    Some(out)
}

/// What became of a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hooked {
    Done,
    /// Its bytes are not the ones expected: left alone.
    Differs,
    /// Outside the image, no room for code near it, or Windows refused.
    Failed(&'static str),
}

/// Hooks `sites` in the image of `size` bytes at `base`; the stubs read the
/// cells at `marks` ([`MARKS_LEN`] bytes). What became of each, said in the
/// log too.
///
/// # Safety
/// The image stays mapped and no thread runs its code yet; `marks` stays
/// valid for as long as the image runs.
pub unsafe fn apply(base: usize, size: usize, marks: usize, sites: &[Site]) -> Vec<Hooked> {
    use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut room: Option<(usize, usize)> = None;
    let mut out = Vec::new();
    for site in sites {
        let done = (|| {
            if site.window.checked_add(site.bytes.len()).is_none_or(|end| end > size) || site.store + site.len > site.bytes.len() {
                return Hooked::Failed("outside the image");
            }
            let window = base + site.window;
            // SAFETY: inside the mapped image (checked above).
            let there = unsafe { std::slice::from_raw_parts(window as *const u8, site.bytes.len()) };
            if there != site.bytes {
                return Hooked::Differs;
            }
            let at = window + site.store;
            if room.is_none() {
                room = near(at).map(|place| (place, 0));
            }
            let Some((place, used)) = room.as_mut() else {
                return Hooked::Failed("no room for code near the title's");
            };
            let to = *place + *used;
            let store = &site.bytes[site.store..site.store + site.len];
            let (Some(body), Some(jump)) = (stub(store, at, to, marks), jump(at, to)) else {
                return Hooked::Failed("not a store of the expected form, or out of reach");
            };
            if *used + body.len() > STUBS_LEN {
                return Hooked::Failed("no room for code near the title's");
            }
            let mut old = 0;
            // SAFETY: our own code memory, which nothing runs yet; then the
            // store, whose page is made writable for it and put back, and
            // which nothing runs either.
            unsafe {
                std::ptr::copy_nonoverlapping(body.as_ptr(), to as *mut u8, body.len());
                if VirtualProtect(at as *const _, site.len, PAGE_EXECUTE_READWRITE, &mut old) == 0 {
                    return Hooked::Failed("Windows would not make it writable");
                }
                std::ptr::write_bytes(at as *mut u8, 0x90, site.len);
                std::ptr::copy_nonoverlapping(jump.as_ptr(), at as *mut u8, jump.len());
                VirtualProtect(at as *const _, site.len, old, &mut old);
                FlushInstructionCache(GetCurrentProcess(), at as *const _, site.len);
            }
            *used += body.len().next_multiple_of(16);
            Hooked::Done
        })();
        let offset = format_args!("{:#x}", site.window + site.store);
        match done {
            Hooked::Done => tracing::info!(site = %offset, "renderer port 2: the title's descriptor record store at title+site patched to tell the layer its slot (VICTORY_PORT_DESCRIPTORS)"),
            Hooked::Differs => tracing::warn!(site = %offset, "renderer port 2: not patched, the title's bytes there are not the ones expected (another build?)"),
            Hooked::Failed(why) => tracing::warn!(site = %offset, why, "renderer port 2: not patched"),
        }
        out.push(done);
    }
    out
}
