// A `str`'s `repeat` past `usize::MAX` bytes panics "capacity overflow" as
// std's `[u8]::repeat` does, a `String` payload raised at its
// `checked_mul`'s `expect`; past `isize::MAX` bytes at `Vec`'s room; and a
// `Vec`'s `extend` of a `TrustedLen` iterator longer than `usize::MAX`, an
// inclusive range of every `usize` or a chain past it, at its
// `extend_trusted`, a `&str` payload, before any is pushed: never JS's own
// `RangeError` of a string's or an array's length, which no payload reads
// (#2487, #2511). A count of a signed integer below 0 is one past
// `usize::MAX` bytes as its `usize`. Text that fits, and empty text of any
// count, repeat as JS's own.
use std::panic;

fn said(r: std::thread::Result<usize>) -> String {
    match r {
        Ok(n) => format!("ok {n}"),
        Err(p) => format!("{:?} {:?}", p.downcast_ref::<&str>(), p.downcast_ref::<String>()),
    }
}

fn indented(indent: i32) -> usize {
    "  ".repeat(indent as usize).len()
}

fn main() {
    println!("{}", said(panic::catch_unwind(|| "ab".repeat(usize::MAX).len())));
    println!("{}", said(panic::catch_unwind(|| String::from("ab").repeat(usize::MAX / 2 + 1).len())));
    println!("{}", said(panic::catch_unwind(|| indented(-1))));
    println!("{}", said(panic::catch_unwind(|| indented(2))));
    println!("{}", said(panic::catch_unwind(|| "".repeat(usize::MAX).len())));
    println!("{}", said(panic::catch_unwind(|| {
        let mut v = vec![1usize];
        v.extend(0..=usize::MAX);
        v.len()
    })));
    println!("{}", said(panic::catch_unwind(|| {
        let mut v = vec![1usize];
        v.extend((0..usize::MAX).chain(0..2));
        v.len()
    })));
    let mut v = vec![1usize];
    v.extend((0..3).chain(0..2));
    v.extend(0..=2);
    println!("{v:?} {}", "ab".repeat(3));
}
