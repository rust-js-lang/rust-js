// What std leaves where an edit panics (#1171, found by the std net): an
// `insert` past the end drops what it took as it unwinds, a `Vec`'s and a
// `VecDeque`'s; a `String`'s `retain` whose closure panics leaves what it
// kept of the characters before, what it hadn't asked of gone too, though
// one that panics before it removed any leaves the text whole, 1.100's
// (ADR 0109, amended). One whose closure can't panic keeps its JS's own
// form.
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct L(u32);

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut v = vec![L(1), L(2)];
    let r = catch_unwind(AssertUnwindSafe(|| v.insert(9, L(9))));
    println!("{} {}", r.is_err(), v.len());
    let mut d = VecDeque::from(vec![L(3)]);
    let r = catch_unwind(AssertUnwindSafe(|| d.insert(5, L(8))));
    println!("{} {}", r.is_err(), d.len());
    let mut s = String::from("héllo wörld");
    let mut n = 0;
    let r = catch_unwind(AssertUnwindSafe(|| {
        s.retain(|c| {
            n += 1;
            if n == 4 {
                panic!("call");
            }
            c != 'l'
        })
    }));
    println!("{} {s:?}", r.is_err());
    let mut whole = String::from("héllo");
    let r = catch_unwind(AssertUnwindSafe(|| {
        whole.retain(|c| {
            if c == 'l' {
                panic!("call");
            }
            true
        })
    }));
    println!("{} {whole:?}", r.is_err());
    let mut t = String::from("a b c");
    t.retain(|c| c != ' ');
    println!("{t}");
}
