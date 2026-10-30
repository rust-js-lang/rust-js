//@ library-differs
// A placeholder's width, precision and sign reach what std's `Display`
// hands them to: what a `Box`, an `Rc` or an `Arc` holds, a `Wrapping`'s
// number, a `Cow<str>`'s or a path's text, and a parse error's message,
// as itself or as a `dyn Error`. A `Utf8Error` writes its message as it
// is, unpadded.
use std::borrow::Cow;
use std::error::Error;
use std::num::Wrapping;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

fn main() {
    let name = Box::new(String::from("ada"));
    let count = Rc::new(7);
    let ratio = Box::new(1.25f64);
    println!("[{:>6}] [{:<4}] [{:.1}] [{:+}] [{:>5}]", name, count, ratio, ratio, Arc::new('x'));
    let cow: Cow<str> = "cow".into();
    let lossy = String::from_utf8_lossy(b"ab");
    println!("[{:>5}] [{:^6}] [{:>4}]", cow, lossy, Path::new("a").display());
    println!("[{:>4}] [{:>6.1}]", Wrapping(3u8), Wrapping(2.75f64));
    let error = "x".parse::<i32>().unwrap_err();
    println!("[{:>32}] [{:.3}]", error, error);
    println!("[{:>24}]", "x".parse::<f64>().unwrap_err());
    let boxed: Box<dyn Error> = Box::new("q".parse::<u8>().unwrap_err());
    println!("[{:>32}]", boxed);
    let invalid = std::str::from_utf8(&[0xff]).unwrap_err();
    println!("[{:>50}]", invalid);
}
