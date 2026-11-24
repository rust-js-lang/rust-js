//! A library whose functions copy a `Copy` struct, and which changes
//! neither it nor its part in place: so it leaves each copy out, `return
//! p`, and its contract records the struct and the part its walk found
//! unchanged, `Inner` of `Pair`'s `left` (#2204).
#[derive(Clone, Copy, Debug)]
pub struct Inner {
    pub n: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Pair {
    pub left: Inner,
    pub tag: u8,
}

pub fn copied(p: &Pair) -> Pair {
    *p
}

pub fn twice(p: Pair) -> (Pair, Pair) {
    (p, p)
}
