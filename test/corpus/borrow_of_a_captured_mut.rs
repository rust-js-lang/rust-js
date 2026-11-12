//@ native: wasm32
// A shared reference to a `&mut` held as its place's own name, a closure's
// capture of it by name, is a handle on that place, the cell its type says
// it is, so `Borrow::borrow(&r)` and `r.as_ref()` read what's in the place:
// they read `.value` of the place itself, `undefined`, and the string's
// length of it threw (#1514). A number's `&mut` and a `String`'s.
use std::borrow::Borrow;

fn main() {
    let mut v: i32 = 1;
    let r = &mut v;
    let mut by = move || {
        let b: &i32 = Borrow::borrow(&r);
        println!("borrowed {b:?}");
        *r += 10;
    };
    by();
    println!("{v:?}");

    let mut s = String::from("a");
    let r = &mut s;
    let mut by = move || {
        let b: &str = r.as_ref();
        println!("as_ref {b} {}", b.len());
        r.push('!');
    };
    by();
    println!("{s}");
}
