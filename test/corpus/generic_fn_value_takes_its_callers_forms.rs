// A generic function whose `&mut T` is generic code's box, or whose `&mut W`
// of a `W` given a `&mut` to an object is a box of that box, as a value
// (#1324, #1322): given to std's `map`, a function of the crate's, a field
// and a concrete `F`, it's called with what its callers hold, and given to
// generic code that calls it as an `F` of its type parameter, with what
// generic code holds. Generic code's `F` of a held `&mut` is refused, as
// is any `&mut` to an object inside its parameters.
use std::io::Write;

fn view<T: ?Sized>(r: &mut T) -> &T {
    &*r
}

fn stamped<W: Write>(w: &mut W) -> &mut W {
    w.write_all(b"s").unwrap();
    w
}

fn stamp<W: Write>(w: &mut W) {
    w.write_all(b"t").unwrap();
}

fn emptied<T: Default>(t: &mut T) {
    std::mem::take(t);
}

fn longest<T: Default>(r: &mut Vec<T>) -> usize {
    r.push(T::default());
    r.len()
}

fn first_len(f: fn(&mut Vec<u8>) -> &Vec<u8>, rows: &mut [Vec<u8>]) -> usize {
    f(&mut rows[0]).len()
}

fn stamp_twice<'b>(f: fn(&mut &'b mut Vec<u8>), w: &mut &'b mut Vec<u8>) {
    f(w);
    f(w);
}

fn concrete(w: &mut Vec<u8>, f: impl Fn(&mut Vec<u8>) -> &Vec<u8>) -> usize {
    f(w).len()
}

fn generic<T, F: Fn(&mut T)>(t: &mut T, f: F) {
    f(t);
}

struct Viewer {
    of: fn(&mut String) -> &String,
}

fn main() {
    let mut v = vec![1u8, 2];
    println!("{}", Some(&mut v).map(view).map_or(0, |v| v.len()));
    let mut rows = vec![vec![1u8], vec![2, 3]];
    let lens: Vec<usize> = rows.iter_mut().map(view).map(|r| r.len()).collect();
    println!("{lens:?}");
    let grown: Vec<usize> = rows.iter_mut().map(longest).collect();
    println!("{grown:?} {rows:?}");
    println!("{}", first_len(view, &mut rows));
    generic(&mut rows[1], emptied);
    generic(&mut rows, |rows| rows.push(vec![9]));
    println!("{rows:?}");
    println!("{}", concrete(&mut v, view));
    let viewer = Viewer { of: view };
    let mut text = String::from("ab");
    println!("{}", (viewer.of)(&mut text).len());

    let mut bytes = Vec::new();
    let mut into = &mut bytes;
    Some(&mut into).map(stamped).unwrap().write_all(b"m").unwrap();
    stamp_twice(stamp, &mut into);
    generic(&mut text, |t| t.push('c'));
    let mut other = Vec::new();
    if let Some(w) = Some(&mut into).map(stamped) {
        *w = &mut other;
    }
    into.write_all(b"o").unwrap();
    println!("{bytes:?} {other:?} {text}");
}
