// `Option<Option<T>>::flatten()` of a value with a destructor moves the
// inner `Option` out and drops nothing, so each value is dropped once,
// where its new owner drops it: of `Some(Some(x))`, `Some(None)` and
// `None`, of a nested `Option` three deep, of a boxed value, flattened
// in place and through a function, and as a panic unwinds past it (#1857).
use std::panic;

#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn flat<T>(o: Option<Option<T>>) -> Option<T> {
    o.flatten()
}

fn main() {
    let o: Option<Option<D>> = Some(Some(D(1)));
    println!("{:?}", o.flatten().map(|d| d.0));
    let kept = Some(Some(D(2))).flatten();
    println!("kept {kept:?}");
    let none: Option<Option<D>> = Some(None);
    println!("{:?}", none.flatten().is_none());
    let empty: Option<Option<D>> = None;
    println!("{:?}", empty.flatten().is_none());
    let deep: Option<Option<Option<D>>> = Some(Some(Some(D(3))));
    let once = deep.flatten();
    println!("once {once:?}");
    let twice = once.flatten();
    println!("twice {twice:?}");
    let boxed = Some(Some(Box::new(D(4)))).flatten();
    println!("boxed {}", boxed.as_ref().map_or(0, |d| d.0));
    println!("{:?}", flat(Some(Some(D(5)))).map(|d| d.0 + 10));
    let r = panic::catch_unwind(|| {
        let held = Some(Some(D(6))).flatten();
        if held.is_some() {
            panic!("past it");
        }
        held
    });
    println!("panicked {}", r.is_err());
    println!("end");
}
