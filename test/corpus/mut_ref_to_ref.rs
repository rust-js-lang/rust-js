// A `&mut` to a shared reference is a cell of it, as one to a number is:
// `*s = &s[1..]` replaces the reference where the `&mut` came from.

fn shorten(s: &mut &str) {
    *s = &s[1..];
}

fn pick<'a>(slot: &mut &'a [i32], other: &'a [i32]) {
    if other.len() > slot.len() {
        *slot = other;
    }
}

struct Cursor<'a> {
    rest: &'a str,
}

fn main() {
    let mut s = "hello";
    shorten(&mut s);
    shorten(&mut s);
    println!("{s}");
    let a = [1, 2];
    let b = [3, 4, 5];
    let mut longest: &[i32] = &a;
    pick(&mut longest, &b);
    println!("{longest:?}");
    let mut words = vec!["a", "bb", "c"];
    for w in words.iter_mut() {
        if w.len() == 1 {
            *w = "one";
        }
    }
    println!("{words:?}");
    let r = &mut s;
    *r = "x";
    println!("{s}");
    let mut cursor = Cursor { rest: "abc" };
    shorten(&mut cursor.rest);
    println!("{}", cursor.rest);
}
