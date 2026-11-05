// Tests of a value's parts read in the order the Rust reads them where
// one may panic or do something: a slice's items, each bounds-checked,
// `s[3]` first, so a short slice panics there as Rust does; a string's
// bytes and chars; only a tuple's parts, the array rust-js makes of it
// (`Known::Part`), are read in the order its pattern names them (#756,
// P0; a binding's `js::get(o, &3.0)`'s getter order is
// lowering-match-paths.test.ts's).

use std::panic::catch_unwind;

fn tuple_alts(t: (u32, u32, u32, u32)) -> bool {
    t.3 == 1 && (t.0 == 2 || t.0 == 3) && (t.1 == 4 || t.1 == 5) && (t.2 == 6 || t.2 == 7)
}

fn slice_alts(s: &[u32]) -> bool {
    s[3] == 1 && (s[0] == 2 || s[0] == 3) && (s[1] == 4 || s[1] == 5) && (s[2] == 6 || s[2] == 7)
}

fn array(a: [u32; 3]) -> bool {
    a[2] == 1 && (a[0] == 2 || a[0] == 3) && a[1] == 4
}

fn bytes(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() > 4 && b[4] == b':' && (b[0] == b'd' || b[0] == b'D') && (b[1] == b'a' || b[1] == b'A')
}

fn char_alts(value: &str) -> bool {
    let c = |i: usize| value.get(i..i + 1);
    c(3) == Some("a")
        && (c(0) == Some("d") || c(0) == Some("D"))
        && (c(1) == Some("a") || c(1) == Some("A"))
        && (c(2) == Some("t") || c(2) == Some("T"))
}

fn main() {
    for t in [(2, 4, 6, 1), (3, 5, 7, 1), (2, 4, 6, 0), (1, 4, 6, 1)] {
        println!("tuple {t:?} {}", tuple_alts(t));
    }
    for s in [&[2, 4, 6, 1][..], &[3, 5, 7, 0], &[2, 4], &[9, 9, 9]] {
        match catch_unwind(|| slice_alts(s)) {
            Ok(b) => println!("slice {s:?} {b}"),
            Err(_) => println!("slice {s:?} panicked"),
        }
    }
    for a in [[2, 4, 1], [3, 4, 1], [2, 5, 1], [2, 4, 0]] {
        println!("array {a:?} {}", array(a));
    }
    for s in ["data:x", "DAta:y", "datax", "dat", "Data:"] {
        println!("bytes {s} {} chars {}", bytes(s), char_alts(s));
    }
}
