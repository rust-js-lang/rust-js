//! A function a consumer returns after a guard's `finally`, React's
//! `shared/noop`.
pub fn noop() -> u32 {
    0
}

/// Another, a consumer's `use` names, as upstream's `import noop from
/// 'shared/noop'` does.
pub fn idle() -> u32 {
    1
}
