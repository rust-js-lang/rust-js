//! A library's statics whose values change in place, an atomic's, which
//! JS holds as `{ value }`: its consumer's read of one is held before a
//! write that follows it, as the library's own is (#1028).
pub mod counters {
    use std::sync::atomic::AtomicU32;

    pub static HITS: AtomicU32 = AtomicU32::new(1);
}

pub use counters::HITS;
