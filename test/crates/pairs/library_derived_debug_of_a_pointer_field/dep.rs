//! A library's type with a derived `Debug` of a pointer field, rand 0.7.3's
//! `ThreadRng`, beside a `Debug` that writes a `dyn Error`, which may be any
//! of its writers, so each writes a piece at a time where a writer of its
//! user's is written into: the pointer's address, std's `pad_integral` of
//! it, its `0x` and its digits (#1420, #2282).
use core::fmt;
use core::ptr::NonNull;

#[derive(Clone, Copy, Debug)]
pub struct ThreadRng {
    rng: NonNull<u64>,
}

pub fn thread_rng() -> ThreadRng {
    ThreadRng {
        rng: NonNull::dangling(),
    }
}

pub struct Error {
    inner: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error {{ inner: {:?} }}", self.inner)
    }
}

pub fn failed(why: &str) -> Error {
    Error { inner: why.into() }
}
