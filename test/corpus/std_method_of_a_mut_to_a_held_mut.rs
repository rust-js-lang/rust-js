//@ library-differs
// A std trait's method called on a `&mut` to a local that holds a `&mut`,
// which std's impl for a `&mut W` passes on to `W`'s (#1309): the write is
// to what the `&mut` it's of points at. A `String` written by
// `fmt::Write`, a `Vec<u8>` by `io::Write`, a reader of bytes read by
// `BufRead` and `Read`, and an iterator stepped, each through a `&mut` held
// in a local, in a struct's field and in an `Option`. Not a method std's
// impl makes of another, or leaves its trait's default: an iterator of the
// crate's own folded through a `&mut` is `try_fold`'s, and found through a
// `&mut` to a `&mut`, the default `find`'s, each of its `next`, not its own
// `fold` or `find`.
use std::fmt::Write as _;
use std::io::{BufRead, Read, Write as _};

struct Count(u32);

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 <= 3).then_some(self.0)
    }
    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, _: F) -> B {
        println!("its own fold");
        init
    }
    fn find<P: FnMut(&u32) -> bool>(&mut self, _: P) -> Option<u32> {
        println!("its own find");
        None
    }
}

struct Out<'a> {
    text: &'a mut String,
}

fn main() {
    let mut s = String::new();
    let mut by = &mut s;
    let r = &mut by;
    r.write_str("a").unwrap();
    write!(r, "b{}", 1).unwrap();
    r.write_char('c').unwrap();
    println!("{s:?}");

    let mut bytes: Vec<u8> = Vec::new();
    let mut held = &mut bytes;
    let w = &mut held;
    w.write_all(b"z").unwrap();
    w.write_all(b"yx").unwrap();
    w.flush().unwrap();
    write!(w, "{}", 7).unwrap();
    println!("{bytes:?}");

    let mut text = String::from("<");
    let mut out = Out { text: &mut text };
    let into = &mut out.text;
    into.write_str("field").unwrap();
    writeln!(into, ">").unwrap();
    println!("{text:?}");

    let mut slot = String::new();
    let mut maybe = Some(&mut slot);
    if let Some(inner) = maybe.as_mut() {
        inner.write_str("some").unwrap();
    }
    println!("{slot:?}");

    let mut data: &[u8] = b"first\nsecond\nrest";
    let mut reader = &mut data;
    let from = &mut reader;
    let mut line = String::new();
    let read = from.read_line(&mut line).unwrap();
    let mut two = [0u8; 2];
    from.read_exact(&mut two).unwrap();
    let mut left = Vec::new();
    from.read_to_end(&mut left).unwrap();
    println!("{read} {line:?} {two:?} {left:?} {}", data.len());

    let mut items = vec![1, 2, 3, 4];
    let mut it = items.iter_mut();
    let mut stepped = &mut it;
    let step = &mut stepped;
    if let Some(x) = step.next() {
        *x += 10;
    }
    if let Some(x) = step.nth(1) {
        *x += 100;
    }
    let rest = step.count();
    println!("{items:?} {rest}");

    let mut count = Count(0);
    let by_ref = &mut count;
    let sum = by_ref.fold(0, |sum, n| sum + n);
    println!("{sum}");

    let mut again = Count(0);
    let mut held = &mut again;
    let finds = &mut held;
    println!("{:?}", finds.find(|&n| n == 2));
}
