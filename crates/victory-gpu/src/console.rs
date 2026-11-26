//! Which console the title runs on. Titles pick settings (resolution,
//! memory budgets) by it, so the user chooses; Series X unless told.

use std::sync::atomic::{AtomicBool, Ordering};

const GIB: usize = 1 << 30;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Console {
    #[default]
    SeriesX,
    SeriesS,
}

impl Console {
    /// Title memory in bytes: (GPU-optimal, standard). Series X in the
    /// advanced configuration; Series S has one range, counted as standard
    /// (7,776 MiB, the published console-mode figure).
    // shortcut: later system updates gave Series S titles more (amount not
    // published here); the title sizes nothing from this figure
    // (it goes by the device type), so update it if a title does.
    ///
    /// `VICTORY_SERIES_S_MEMORY=x`: Series S reports Series X's amounts (a
    /// test: Series S at a render scale above 2/3 stopped at the title's own
    /// out-of-memory check, an 8 MB block its pools could not give; this
    /// says whether those pools are sized from the memory reported).
    pub fn title_memory(self) -> (usize, usize) {
        static BIGGER: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let bigger = *BIGGER.get_or_init(|| {
            let on = std::env::var_os("VICTORY_SERIES_S_MEMORY").is_some_and(|v| v == "x");
            if on {
                tracing::warn!("Series S reports Series X's title memory (VICTORY_SERIES_S_MEMORY=x)");
            }
            on
        });
        match self {
            Console::SeriesX => (10 * GIB, 7 * GIB / 2),
            Console::SeriesS if bigger => (10 * GIB, 7 * GIB / 2),
            Console::SeriesS => (0, 7_776 << 20),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Console::SeriesX => "Xbox Series X",
            Console::SeriesS => "Xbox Series S",
        }
    }
}

static SERIES_S: AtomicBool = AtomicBool::new(false);

/// Set before the title starts.
pub fn set(console: Console) {
    SERIES_S.store(console == Console::SeriesS, Ordering::Relaxed);
}

pub fn current() -> Console {
    if SERIES_S.load(Ordering::Relaxed) {
        Console::SeriesS
    } else {
        Console::SeriesX
    }
}
