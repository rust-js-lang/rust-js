// A `&mut` to a local that holds a `&mut` to an object, given to generic
// code that gives it back, `&mut W` of `&mut by` (#1303): what's given back
// is the caller's, a `&mut` to what `by` holds, written through, replaced
// whole and given back again; a `Vec<u8>` written, a `String` pushed to and
// a struct changed, through a function value too, and given by a closure
// that holds the `&mut` as its parameter.
use std::io::Write;

fn keep<W: Write>(w: &mut W) -> &mut W {
    w
}

fn same<T>(t: &mut T) -> &mut T {
    t
}

#[derive(Debug)]
struct Tally {
    seen: u32,
}

fn main() {
    let mut v = Vec::new();
    let mut by = &mut v;
    keep(&mut by).write_all(b"k").unwrap();
    keep(keep(&mut by)).write_all(b"j").unwrap();
    let mut other = Vec::new();
    *keep(&mut by) = &mut other;
    by.write_all(b"o").unwrap();
    println!("{v:?} {other:?}");

    let mut text = String::from("<");
    let mut held = &mut text;
    same(&mut held).push('a');
    same(same(&mut held)).push_str("bc");
    println!("{text}");

    let mut tally = Tally { seen: 0 };
    let mut counted = &mut tally;
    same(&mut counted).seen += 2;
    let back = same(&mut counted);
    back.seen *= 10;
    println!("{tally:?}");

    let f = keep::<&mut Vec<u8>>;
    let mut bytes = Vec::new();
    let mut into = &mut bytes;
    f(&mut into).write_all(b"f").unwrap();
    let g = |w: &mut &mut Vec<u8>| keep(w).write_all(b"g").unwrap();
    g(&mut into);
    let h = |w| keep::<&mut Vec<u8>>(w);
    h(&mut into).write_all(b"h").unwrap();
    println!("{bytes:?}");
}
