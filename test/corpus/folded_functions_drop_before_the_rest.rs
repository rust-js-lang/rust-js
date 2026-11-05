// What a fold by value moves into it, a consumer's closure or an adapter's,
// drops before what's left of what it folds, as std's `fold(self, init, f)`
// drops `f` first (#734, #754): through an adapter whose fold reaches the
// crate's own `fold`; of a `step_by()`, whose source is stepped to its end
// by its `nth`; of a `map()` and a `filter()` of a `Vec`'s items folded from
// the back, and of a `Vec`'s reversed or mapped; and of std's `Rev`, `rfold`
// of a range's `fold`, with no panic.
// And a chain under a map that holds a closure, each side closed in turn,
// as it's done, from either end.
struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// Counts down from `left`, with its own `fold`, `nth` and `next_back`.
struct Own {
    left: u32,
    _held: D,
}

impl Iterator for Own {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }

    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

fn own(left: u32, name: &'static str) -> Own {
    Own { left, _held: D(name) }
}

/// Counts down from `left`, with std's steps alone.
struct Plain {
    left: u32,
    _held: D,
}

impl Iterator for Plain {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
}

fn plain(left: u32, name: &'static str) -> Plain {
    Plain { left, _held: D(name) }
}

/// `Plain`, stepped from its back too.
struct Backs(Plain);

impl Iterator for Backs {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0.next()
    }
}

impl DoubleEndedIterator for Backs {
    fn next_back(&mut self) -> Option<u32> {
        self.0.next()
    }
}

fn backs(left: u32, name: &'static str) -> Backs {
    Backs(plain(left, name))
}

fn zero(_guard: &D) -> u32 {
    0
}

fn reaching_its_own_fold() {
    let g = D("f");
    let n = own(4, "own").take(3).fold(0, move |a, x| a + x + zero(&g));
    println!("take {n}");
    let g = D("f");
    let n = own(4, "own").skip(1).fold(0, move |a, x| a + x + zero(&g));
    println!("skip {n}");
    let g = D("f");
    let n = own(3, "own").peekable().fold(0, move |a, x| a + x + zero(&g));
    println!("peekable {n}");
    let g = D("f");
    let n = own(3, "own").fuse().fold(0, move |a, x| a + x + zero(&g));
    println!("fuse {n}");
    let g = D("f");
    own(2, "own").enumerate().for_each(move |(i, x)| {
        let _g = &g;
        println!("item {i} {x}");
    });
    let g = D("f");
    let n = own(2, "own")
        .chain(own(1, "own2"))
        .fold(0, move |a, x| a + x + zero(&g));
    println!("chain {n}");
}

fn stepped_to_its_end() {
    let g = D("map");
    let n = own(5, "own")
        .step_by(2)
        .map(move |x| x + zero(&g))
        .fold(0, |a, x| a + x);
    println!("step_by fold {n}");
    let g = D("map");
    let s: String = own(5, "own")
        .step_by(2)
        .map(move |x| char::from(b'a' + (x + zero(&g)) as u8))
        .collect();
    println!("step_by string {s}");
}

fn folded_from_the_back() {
    let g = D("map");
    let h = D("f");
    let n = vec![D("x"), D("y")]
        .into_iter()
        .map(move |d| d.0.len() as u32 + zero(&g))
        .rfold(0, move |a, x| a + x + zero(&h));
    println!("map rfold {n}");
    let g = D("filter");
    let h = D("f");
    let n = own(4, "own")
        .filter(move |&x| x % 2 == 1 || zero(&g) > 0)
        .rfold(0, move |a, x| a + x + zero(&h));
    println!("filter rfold {n}");
}

fn folded_from_the_back_as_it_panics() {
    let g = D("map");
    let h = D("f");
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        vec![D("x"), D("y"), D("z")]
            .into_iter()
            .map(move |d| d.0.len() as u32 + zero(&g))
            .rfold(0, move |a, x| {
                let _h = &h;
                assert!(a == 0, "consumer");
                a + x
            })
    }));
    println!("map rfold panicked {}", r.is_err());
}

fn of_a_vec_reversed() {
    let g = D("f");
    vec![D("x"), D("y")]
        .into_iter()
        .rev()
        .enumerate()
        .for_each(move |(i, d)| {
            let _g = &g;
            println!("item {i} {}", d.0);
        });
    let g = D("f");
    let n = vec![D("x"), D("y")]
        .into_iter()
        .map(|d| d.0.len() as u32)
        .rfold(0, move |a, x| a + x + zero(&g));
    println!("vec map rfold {n}");
    let g = D("f");
    let n = vec![D("x"), D("y")]
        .into_iter()
        .rev()
        .rfold(0, move |a, d| a + d.0.len() as u32 + zero(&g));
    println!("vec rev rfold {n}");
}

fn reversed_without_a_panic() {
    let g = D("f");
    let n = (1..6u32).rev().rfold(0, move |a, x| a + x + zero(&g));
    println!("range rev rfold {n}");
    let g = D("f");
    let n = (1..6u32).inspect(|_| ()).rfold(0, move |a, x| a + x + zero(&g));
    println!("inspect rfold {n}");
    let g = D("f");
    let n = (1..4u32).flat_map(|x| 0..x).rfold(0, move |a, x| a + x + zero(&g));
    println!("flat_map rfold {n}");
    let g = D("f");
    let n = vec![vec![1u32, 2], vec![3]]
        .into_iter()
        .flatten()
        .rfold(0, move |a, x| a + x + zero(&g));
    println!("flatten rfold {n}");
}

fn chained_under_a_map() {
    let g = D("map");
    let n = plain(2, "a")
        .chain(plain(1, "b"))
        .map(move |x| x + zero(&g))
        .fold(0, |a, x| a + x);
    println!("chain map fold {n}");
    let g = D("filter");
    let n = plain(2, "a")
        .step_by(1)
        .chain(plain(1, "b"))
        .filter(move |_| zero(&g) == 0)
        .fold(0, |a, x| a + x);
    println!("chain filter fold {n}");
}

// Of sides that know their back, each closed as it's done.
fn chained_from_either_end() {
    let g = D("map");
    let s: String = backs(2, "a")
        .chain(vec![1u32, 2])
        .map(move |x| char::from(b'a' + (x + zero(&g)) as u8))
        .collect();
    println!("chain string {s}");
}

fn main() {
    reaching_its_own_fold();
    stepped_to_its_end();
    folded_from_the_back();
    folded_from_the_back_as_it_panics();
    of_a_vec_reversed();
    reversed_without_a_panic();
    chained_under_a_map();
    chained_from_either_end();
}
