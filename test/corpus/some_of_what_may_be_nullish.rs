// `Some(v)` of a value whose JS may be `undefined` or `null` is boxed, as
// `Some(())` is (ADR 0051), so it isn't read as `None`: a standard stream
// or its lock, which is nothing, a disabled `Backtrace`, a write's result,
// whose `Ok` is nothing, and a raw pointer, which `0 as *const T` makes
// `null` (#1010). And a `Vec`'s or a stream's `write_all` as a value is
// `Ok`, where its JS gave the length its `push` did, read as an `Err`.
use std::backtrace::Backtrace;
use std::fmt::{self, Write as _};
use std::io::{self, Write};

fn written(w: &mut Vec<u8>) -> Option<io::Result<()>> {
    Some(w.write_all(b"ab"))
}

fn shown() -> Option<fmt::Result> {
    let mut s = String::new();
    Some(write!(s, "{}", 5))
}

fn main() {
    let out = Some(io::stdout());
    println!("{}", out.is_some());
    let err = Some(io::stderr());
    println!("{}", err.is_some());
    let lock = Some(io::stdout().lock());
    println!("{}", lock.is_some());
    let mut v = Vec::new();
    let d = written(&mut v);
    println!("{:?} {:?} {:?}", d.is_some(), v, d);
    let w = v.write_all(b"c");
    println!("{:?} {:?}", w, w.is_ok());
    let mut stdout = io::stdout();
    let o = Some(stdout.write_all(b""));
    println!("{:?}", o);
    println!("{:?}", shown());
    let r: Option<fmt::Result> = Some(Ok(()));
    println!("{:?}", r);
    let b = Some(Backtrace::disabled());
    println!("{}", b.is_some());
    let results: Vec<Option<io::Result<()>>> = vec![Some(Ok(())), None];
    println!("{:?}", results.iter().map(|o| o.is_some()).collect::<Vec<_>>());
    let null = Some(0 as *const u8);
    println!("{} {}", null.is_some(), null.unwrap().is_null());
    let pointers = [Some(std::ptr::null::<u32>()), None];
    println!("{:?}", pointers.map(|p| p.is_some()));
    let unit = Some(());
    println!("{:?}", unit);
}
