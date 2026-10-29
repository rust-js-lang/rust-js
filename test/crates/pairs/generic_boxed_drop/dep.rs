//! A library's generic function that holds what it's given in a `Vec`,
//! `vec![t]`'s box, which its consumer compiles from its optimized MIR,
//! given a type with a destructor (#378); and a generic method, which it
//! can't yet.
pub fn hold<T>(t: T) -> u32 {
    let v = vec![t];
    println!("held");
    v.len() as u32
}

pub struct Bag<T> {
    pub items: Vec<T>,
}

impl<T> Bag<T> {
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

/// What it's given, kept only if asked, checked first where it isn't: its
/// drop flag, its optimized MIR's, known where it's tested (#381).
pub fn given<T>(t: T, keep: bool, check: impl Fn()) -> Option<T> {
    let held = t;
    if keep {
        return Some(held);
    }
    check();
    None
}
