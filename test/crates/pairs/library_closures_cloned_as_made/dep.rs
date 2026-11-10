//! A library's closures a consumer clones, behind an `impl .. + Clone`:
//! each cloned as its library made it (#1202). `lengths`' closure writes
//! nothing, so its library makes it a plain function, whose clone is
//! itself, though rustc infers it `FnMut`, the bound `map` gives it;
//! `running`'s and `counter`'s write what they took, so each is made by a
//! function of what it holds, which a clone makes again.
pub fn lengths(v: Vec<String>) -> impl Iterator<Item = usize> + Clone {
    v.into_iter().map(|s| s.len())
}

pub fn running(v: Vec<u32>) -> impl Iterator<Item = u32> + Clone {
    let mut total = 0;
    v.into_iter().map(move |x| {
        total += x;
        total
    })
}

pub fn counter() -> impl FnMut() -> u32 + Clone {
    let mut n = 0;
    move || {
        n += 1;
        n
    }
}

pub fn scaled(v: &[u32], by: u32) -> impl Iterator<Item = u32> + Clone + '_ {
    v.iter().map(move |x| x * by)
}

/// What `numbered` hides is a private function's `impl .. + Clone`, whose
/// closure its library decides of too, revealed through both.
pub fn numbered(v: Vec<u32>) -> impl Iterator<Item = u32> + Clone {
    weighted(v)
}

fn weighted(v: Vec<u32>) -> impl Iterator<Item = u32> + Clone {
    let mut seen = 0;
    v.into_iter().map(move |x| {
        seen += 1;
        x * seen
    })
}
