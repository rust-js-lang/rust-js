//! A library's public trait, whose generic method its own code calls with a
//! `u32` alone: a consumer may call it with what isn't its own copy, so it
//! takes a copy function, its callers unseen (#1233).
pub trait Pick {
    fn pick<T: Copy>(&self, a: T) -> T;
}

pub struct Same;

impl Pick for Same {
    fn pick<T: Copy>(&self, a: T) -> T {
        a
    }
}

pub fn own() -> u32 {
    Same.pick(3u32)
}
