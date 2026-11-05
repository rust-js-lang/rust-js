// A consumer's own closure that holds a value with a destructor, given by
// value to `fold`, `rfold`, `for_each` or `reduce`, drops before what's
// left of what it consumes, as `fold(self, init, f)` drops `f` first: the
// crate's iterator with a `Drop`, an adapter's rest, a chain's second side;
// a flatten's after its parts, which it folds by `&mut`; and as a panic in
// it unwinds (#734).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
    }
}

struct It {
    left: u32,
    _held: D,
}

impl It {
    fn new(left: u32, name: &'static str) -> It {
        It { left, _held: D(name) }
    }
}

impl Iterator for It {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
}

impl DoubleEndedIterator for It {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

// What reads the guard a closure holds, so it's moved in whole.
fn zero(_guard: &D) -> u32 {
    0
}

fn case(name: &str, run: impl FnOnce(D) -> u32) {
    print!("{name}: ");
    let r = catch_unwind(AssertUnwindSafe(|| run(D("f"))));
    println!("| {r:?}");
}

fn main() {
    case("fold", |g| It::new(3, "it").fold(0, move |a, x| a + x + zero(&g)));
    case("for_each", |g| {
        It::new(2, "it").for_each(move |x| assert!(x + zero(&g) < 9));
        0
    });
    case("rfold", |g| It::new(3, "it").rfold(0, move |a, x| a + x + zero(&g)));
    case("reduce", |g| {
        It::new(3, "it").reduce(move |a, x| a + x + zero(&g)).unwrap_or(0)
    });
    case("skip", |g| {
        It::new(3, "it").skip(1).fold(0, move |a, x| a + x + zero(&g))
    });
    case("map", |g| {
        It::new(3, "it").map(|x| x * 2).fold(0, move |a, x| a + x + zero(&g))
    });
    case("take", |g| {
        It::new(3, "it").take(2).fold(0, move |a, x| a + x + zero(&g))
    });
    case("chain", |g| {
        It::new(2, "it")
            .chain(It::new(1, "it2"))
            .fold(0, move |a, x| a + x + zero(&g))
    });
    case("taken past its end", |g| {
        It::new(3, "it").take(5).fold(0, move |a, x| a + x + zero(&g))
    });
    case("fused", |g| {
        It::new(3, "it").fuse().fold(0, move |a, x| a + x + zero(&g))
    });
    case("enumerate", |g| {
        It::new(2, "it")
            .enumerate()
            .fold(0, move |a, (i, x)| a + i as u32 + x + zero(&g))
    });
    case("step_by", |g| {
        It::new(4, "it").step_by(2).fold(0, move |a, x| a + x + zero(&g))
    });
    case("peekable", |g| {
        It::new(3, "it").peekable().fold(0, move |a, x| a + x + zero(&g))
    });
    case("filter", |g| {
        It::new(3, "it")
            .filter(|&x| x > 0)
            .fold(0, move |a, x| a + x + zero(&g))
    });
    case("zip", |g| {
        It::new(3, "it")
            .zip(0..2)
            .fold(0, move |a, (x, y)| a + x + y + zero(&g))
    });
    case("rev", |g| It::new(3, "it").rev().fold(0, move |a, x| a + x + zero(&g)));
    case("skip_while", |g| {
        It::new(3, "it")
            .skip_while(|&x| x > 1)
            .fold(0, move |a, x| a + x + zero(&g))
    });
    case("flatten", |g| {
        vec![It::new(2, "a"), It::new(1, "b")]
            .into_iter()
            .flatten()
            .fold(0, move |a, x| a + x + zero(&g))
    });
    case("a vec's rest", |g| {
        vec![D("x"), D("y")]
            .into_iter()
            .take(1)
            .fold(0, move |a, _| a + 1 + zero(&g))
    });
    case("by_ref", |g| {
        let mut it = It::new(3, "it");
        let n = it.by_ref().take(2).fold(0, move |a, x| a + x + zero(&g));
        print!("| ");
        n
    });
    case("a panic in the closure", |g| {
        It::new(3, "it").fold(0, move |a, x| {
            assert!(x != 1, "one");
            a + x + zero(&g)
        })
    });
    case("a panic in reduce", |g| {
        It::new(3, "it")
            .reduce(move |a, x| {
                assert!(x != 0, "zero");
                a + x + zero(&g)
            })
            .unwrap_or(0)
    });
}
