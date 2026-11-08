use std::cmp::Ordering;

/// Its `partial_cmp` orders the other way from its `cmp`, saying so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Twisted(pub u32);

impl PartialOrd for Twisted {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        println!("  twisted partial_cmp");
        Some(other.0.cmp(&self.0))
    }
}

impl Ord for Twisted {
    fn cmp(&self, other: &Self) -> Ordering {
        println!("  twisted cmp");
        self.0.cmp(&other.0)
    }
}

/// Derived whole, in one `#[derive]`, of a type without parameters:
/// rustc's derive writes its `partial_cmp` as `Some(Ord::cmp(self,
/// other))`, which asks its field's `cmp` (#964).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Whole(pub Twisted);

/// Derived in two `#[derive]`s: its `partial_cmp` asks its field's
/// `partial_cmp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[derive(Ord)]
pub struct Split(pub Twisted);

/// Hand-written as clippy asks, `Some(self.cmp(other))`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plain(pub Twisted);

impl PartialOrd for Plain {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Plain {
    fn cmp(&self, other: &Self) -> Ordering {
        println!("  plain cmp");
        self.0.cmp(&other.0)
    }
}
