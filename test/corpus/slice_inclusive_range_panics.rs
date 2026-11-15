// An inclusive range past a slice's end panics as Rust 1.100's core says:
// its own end, "range end index 3" of `..=3` on three items, not the
// exclusive end one past it (#1935). An end below the length is checked
// as the exclusive range one past it, so a start past that end says "ends
// at" the exclusive end. Text says its end's byte, after its start's
// boundary. Each panic's message is printed: of indexing a slice, a `&mut`
// of one, its copy, text and a `&mut str`, and of what `slice::range`
// checks, `drain`, `splice`, `copy_within`, `extract_if`,
// `extend_from_within` and a `String`'s edits; a `get`'s `None` too.
use std::any::Any;
use std::collections::VecDeque;
use std::hint::black_box;
use std::panic::{self, AssertUnwindSafe};

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn main() {
    let v = [1u32, 2, 3];
    let s = "abc";
    let n = |x: usize| black_box(x);
    check("..=2", || v[..=n(2)].len());
    check("..=3", || v[..=n(3)].len());
    check("..=5", || v[..=n(5)].len());
    check("1..=3", || v[n(1)..=n(3)].len());
    check("4..=5", || v[n(4)..=n(5)].len());
    check("3..=1", || v[n(3)..=n(1)].len());
    check("2..=0", || v[n(2)..=n(0)].len());
    check("3..=2", || v[n(3)..=n(2)].len());
    check("to_vec ..=3", || v[..=n(3)].to_vec());
    check("get ..=3", || v.get(..=n(3)).map(|p| p.len()));
    check("mut ..=3", || {
        let mut w = [1u32, 2, 3];
        let part = &mut w[..=n(3)];
        part[0] = 9;
        part.len()
    });
    check("mut 1..=2", || {
        let mut w = [1u32, 2, 3];
        let part = &mut w[n(1)..=n(2)];
        part[0] = 9;
        w
    });
    check("reverse ..=3", || {
        let mut w = [1u32, 2, 3];
        w[..=n(3)].reverse();
        w
    });
    check("get_mut ..=3", || {
        let mut w = [1u32, 2, 3];
        w.get_mut(..=n(3)).map(|p| p.len())
    });
    check("vec ..=3", || vec![1u32, 2, 3][..=n(3)].len());
    check("drain ..=3", || {
        let mut w = vec![1u32, 2, 3];
        w.drain(..=n(3)).count()
    });
    check("drain 1..=2", || {
        let mut w = vec![1u32, 2, 3];
        let taken: Vec<u32> = w.drain(n(1)..=n(2)).collect();
        (taken, w)
    });
    check("drain 5..=3", || {
        let mut w = vec![1u32, 2, 3];
        w.drain(n(5)..=n(3)).count()
    });
    check("splice ..=3", || {
        let mut w = vec![1u32, 2, 3];
        w.splice(..=n(3), [7, 8]).count()
    });
    check("copy_within ..=3", || {
        let mut w = [1u32, 2, 3];
        w.copy_within(..=n(3), 0);
        w
    });
    check("extract_if ..=3", || {
        let mut w = vec![1u32, 2, 3];
        w.extract_if(..=n(3), |x| *x > 1).count()
    });
    check("extend_from_within ..=3", || {
        let mut w = vec![1u32, 2, 3];
        w.extend_from_within(..=n(3));
        w
    });
    check("deque range_mut ..=3", || {
        let mut d = VecDeque::from([1u32, 2, 3]);
        d.range_mut(..=n(3)).count()
    });
    check("text ..=2", || s[..=n(2)].len());
    check("text ..=3", || s[..=n(3)].len());
    check("text 1..=3", || s[n(1)..=n(3)].len());
    check("text 4..=5", || s[n(4)..=n(5)].len());
    check("text 3..=1", || s[n(3)..=n(1)].len());
    check("text get ..=3", || s.get(..=n(3)));
    check("text inside a char ..=5", || "h\u{e9}llo"[n(2)..=n(5)].len());
    check("text inside a char ..=6", || "h\u{e9}llo"[n(2)..=n(6)].len());
    check("text inside a char ..=1", || "h\u{e9}llo"[..=n(1)].len());
    check("mut text ..=3", || {
        let mut t = String::from("abc");
        let part = &mut t[..=n(3)];
        part.make_ascii_uppercase();
        t
    });
    check("mut text 1..=2", || {
        let mut t = String::from("abc");
        t[n(1)..=n(2)].make_ascii_uppercase();
        t
    });
    check("text get_mut ..=3", || {
        let mut t = String::from("abc");
        t.get_mut(..=n(3)).map(|p| p.len())
    });
    check("string drain ..=3", || {
        let mut t = String::from("abc");
        t.drain(..=n(3)).count()
    });
    check("string drain 1..=1", || {
        let mut t = String::from("abc");
        let taken: String = t.drain(n(1)..=n(1)).collect();
        (taken, t)
    });
    check("replace_range ..=3", || {
        let mut t = String::from("abc");
        t.replace_range(..=n(3), "z");
        t
    });
    check("string extend_from_within ..=3", || {
        let mut t = String::from("abc");
        t.extend_from_within(..=n(3));
        t
    });
}
