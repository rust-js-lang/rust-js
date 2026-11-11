//! A library that re-exports std's items, by globs, as zerocopy's
//! `core_reexport` does, and by name: each exports nothing, being in no JS
//! module (difference 50, #1315).
pub mod core_reexport {
    pub use core::*;

    pub mod mem {
        pub use core::mem::*;
    }
}

pub mod named {
    pub use core::mem::swap;
}

pub fn twice(n: u32) -> u32 {
    n * 2
}
