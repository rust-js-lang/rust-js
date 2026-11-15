// What's moved into a std call that panics is dropped as the panic unwinds
// out of it, before the caller's own locals: a `splice` whose range is past
// the end drops its replacement (#1936), as does an `insert` past the end
// its item; a replacement that panics as it's stepped, and an `extend` of
// one, drop what they made and what's left. Each run's drops are printed.
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

#[derive(Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn loud_or_panic(i: u32) -> Loud {
    if i == 1 {
        panic!("made {}", i);
    }
    Loud(10 + i)
}

fn splice_given<I: IntoIterator<Item = Loud>>(w: &mut Vec<Loud>, at: std::ops::Range<usize>, with: I) -> usize {
    w.splice(at, with).count()
}


fn main() {
    let n = |x: usize| black_box(x);
    check("splice past the end", || {
        let mut w = vec![Loud(1), Loud(2), Loud(3)];
        w.splice(n(1)..n(4), [Loud(7)]).count()
    });
    check("splice through past the end", || {
        let mut w = vec![Loud(1), Loud(2), Loud(3)];
        w.splice(n(1)..=n(3), [Loud(7), Loud(8)]).count()
    });
    check("splice starting past its end", || {
        let mut w = vec![Loud(1), Loud(2)];
        w.splice(n(2)..n(1), vec![Loud(7)]).count()
    });
    check("splice in range", || {
        let mut w = vec![Loud(1), Loud(2), Loud(3)];
        w.splice(n(1)..n(2), [Loud(7)]).count();
        w.len()
    });
    check("splice of a replacement that panics", || {
        let mut w = vec![Loud(1), Loud(2), Loud(3)];
        w.splice(n(0)..n(1), (0..3).map(loud_or_panic)).count()
    });
    check("splice of a replacement that panics, kept to its end", || {
        let mut w = vec![Loud(1), Loud(2), Loud(3)];
        drop(w.splice(n(1)..n(3), (0..3).map(loud_or_panic)));
        w.len()
    });
    check("extend of an iterator that panics", || {
        let mut w = vec![Loud(1)];
        w.extend((0..3).map(loud_or_panic));
        w.len()
    });
    check("insert past the end", || {
        let mut w = vec![Loud(1), Loud(2)];
        w.insert(n(5), Loud(9));
        w.len()
    });
    check("deque insert past the end", || {
        let mut d = VecDeque::from([Loud(1), Loud(2)]);
        d.insert(n(5), Loud(9));
        d.len()
    });
    check("splice of a given replacement past the end", || {
        let mut w = vec![Loud(1), Loud(2)];
        splice_given(&mut w, n(1)..n(5), vec![Loud(7)])
    });
    check("splice of a given replacement in range", || {
        let mut w = vec![Loud(1), Loud(2)];
        splice_given(&mut w, n(1)..n(2), vec![Loud(7)]);
        w.len()
    });
}
