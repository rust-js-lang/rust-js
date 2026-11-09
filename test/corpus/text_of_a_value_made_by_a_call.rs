// A value whose text doesn't read it, `()`'s `()`, a `Weak`'s `(Weak)`, an
// error's message, is still made before it's shown, where making it does
// something: the call that gives it prints, or panics (#1063).
use std::fmt;
use std::rc::{Rc, Weak};

fn give() -> &'static () {
    println!("give ran");
    &()
}

fn pass<T>(t: T) -> T {
    println!("pass ran");
    t
}

fn weak() -> Weak<u32> {
    println!("weak ran");
    Rc::downgrade(&Rc::new(5))
}

fn decoded(units: &[u16]) -> Result<String, std::string::FromUtf16Error> {
    println!("decoding {} units", units.len());
    String::from_utf16(units)
}

// Given the options its own `Formatter` was given.
struct Shown;

impl fmt::Debug for Shown {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(give(), f)
    }
}

fn main() {
    println!("{:?}", give());
    println!("{:?}", *give());
    println!("{:?}", pass(&()));
    println!("[{:>4?}]", pass(&()));
    println!("[{:.1?}]", give());
    println!("{:?}", weak());
    let given = format!("[{:>4?}] [{:?}]", Shown, Shown);
    println!("{given}");
    let shown = format!("{:?} and {:?}", pass(&()), weak());
    println!("{shown}");
    println!("{}", decoded(&[0xDD1E]).unwrap_err());
    let checked = std::panic::catch_unwind(|| {
        println!("{}", decoded(&[104, 105]).unwrap_err());
    });
    println!("unwrap_err of Ok panicked: {}", checked.is_err());
}
