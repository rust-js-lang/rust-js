// A closure that gives back a `fmt::Result` that may be an `Err`, of its
// own, `?` of a function's, or what a generic `F` or a `dyn Fn` gives: its
// call caught as a function's is, `$writeTry`, its result a value, where
// the closure's maker was taken for the one that fails and its throw went
// past what read it; and a function a variable holds, called (#1042).
use std::fmt::{self, Write};

fn made(fail: bool) -> fmt::Result {
    if fail { Err(fmt::Error) } else { Ok(()) }
}

fn run<F: Fn(bool) -> fmt::Result>(f: F) -> fmt::Result {
    f(true)
}

fn main() {
    let checked = || -> fmt::Result {
        made(true)?;
        Ok(())
    };
    println!("{:?}", checked());
    let failing = |fail: bool| if fail { Err(fmt::Error) } else { Ok(()) };
    println!("{:?} {:?}", failing(true), failing(false));
    println!("{}", failing(true).is_err());
    println!("{:?}", run(failing));
    println!("{:?}", run(|fail| made(fail)));
    let mut s = String::new();
    let mut put = |n: u32| -> fmt::Result {
        if n > 9 {
            return Err(fmt::Error);
        }
        write!(s, "{n}")
    };
    println!("{:?} {:?}", put(3), put(30));
    println!("{s:?}");
    fn named() -> fmt::Result {
        made(true)?;
        Ok(())
    }
    let held = named;
    println!("{:?}", held());
    let boxed: Box<dyn Fn() -> fmt::Result> = Box::new(|| Err(fmt::Error));
    println!("{:?}", boxed());
}
