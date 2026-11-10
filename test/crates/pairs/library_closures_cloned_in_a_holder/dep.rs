//! A library's closures a consumer clones or copies inside what holds
//! them, an `Option`, a tuple, a `Vec` and a struct of the library's: each
//! a closure that changes what it took, so each clone and copy counts on
//! its own (#1211).
#[derive(Clone, Copy)]
pub struct Ticker<F> {
    pub tick: F,
}

pub fn counter() -> impl FnMut() -> u32 + Copy {
    let mut n = 0;
    move || {
        n += 1;
        n
    }
}

pub fn maybe_counter() -> Option<impl FnMut() -> u32 + Clone> {
    let mut n = 10;
    Some(move || {
        n += 1;
        n
    })
}

pub fn ticker() -> Ticker<impl FnMut() -> u32 + Copy> {
    Ticker { tick: counter() }
}

pub fn said() -> (impl FnMut() -> String + Clone, u8) {
    let mut text = String::new();
    let say = move || {
        text.push('x');
        text.clone()
    };
    (say, 1)
}
