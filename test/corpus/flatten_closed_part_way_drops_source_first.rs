// A `flatten()` or a `flat_map` dropped part-way drops its source first,
// then the inner iterator it's in, as Rust drops a `Flatten`'s fields in
// order, of a source that's an adapter too, a `map`'s or a `filter`'s of
// what has a destructor, and of one with nothing to drop, a range's
// (#644); one stepped from its back knows its back, and one folded,
// `count()`'s, folds its source by the crate's own `fold`, as std's does;
// its source ended is dropped then, as std's `Fuse` drops it; and one
// collected into a `String` is folded, as `String::extend` steps it; and
// its inner iterator is dropped though its source's drop panics.
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn mapped() {
    let mut it = vec![D(1), D(2)].into_iter().map(|x| vec![D(x.0 + 10), x]).flatten();
    let first = it.next();
    println!("first {}", first.as_ref().map_or(0, |d| d.0));
    drop(first);
    drop(it);
    println!("mapped end");
}
fn flat_mapped() {
    let mut it = vec![D(3), D(4)]
        .into_iter()
        .filter(|d| d.0 > 0)
        .flat_map(|x| vec![D(x.0 + 10), x]);
    let first = it.nth(0);
    println!("first {}", first.as_ref().map_or(0, |d| d.0));
    drop(first);
    drop(it);
    println!("flat mapped end");
}
fn broken_off() {
    for d in vec![D(5), D(6)].into_iter().map(|x| vec![D(x.0 + 10), x]).flatten() {
        println!("got {}", d.0);
        break;
    }
    println!("broken off end");
}
fn of_a_range() {
    let mut it = (0..3).map(|i| vec![D(i + 20), D(i + 30)]).flatten();
    let first = it.next();
    println!("first {}", first.as_ref().map_or(0, |d| d.0));
    drop(first);
    drop(it);
    println!("of a range end");
}
/// Gives `D`s up to 3, folding by its own `fold`.
struct Folds(u32);
impl Iterator for Folds {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        self.0 += 1;
        (self.0 < 3).then(|| D(self.0))
    }
    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(d) = self.next() {
            acc = f(acc, d);
        }
        acc
    }
}
fn by_its_own_fold() {
    let n = Folds(0).map(|x| vec![D(x.0 + 40), x]).flatten().count();
    println!("by its own fold {n}");
}
/// Gives `D(51)` and `D(52)`, and prints as it's dropped.
struct Src(u32);
impl Iterator for Src {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        self.0 += 1;
        (self.0 < 3).then(|| D(self.0 + 50))
    }
}
impl Drop for Src {
    fn drop(&mut self) {
        println!("drop src");
    }
}
fn source_ended() {
    let mut peeked = Src(0).map(|x| vec![x]).peekable();
    println!("peeked {}", peeked.peek().map_or(0, |v| v.len()));
    let v: Vec<D> = peeked.flatten().collect();
    println!("collected {}", v.len());
}
/// Prints as it's dropped, and panics as `Boom(2)` is.
struct Boom(u32);
impl Drop for Boom {
    fn drop(&mut self) {
        println!("drop boom {}", self.0);
        if self.0 == 2 {
            panic!("boom");
        }
    }
}
fn source_drop_panics() {
    let r = std::panic::catch_unwind(|| {
        let mut it = vec![Boom(1), Boom(2), Boom(3)].into_iter().map(|x| vec![Boom(x.0 + 10), x]).flatten();
        let first = it.next();
        println!("first {}", first.as_ref().map_or(0, |b| b.0));
        drop(first);
        drop(it);
    });
    println!("source drop panics {}", r.is_err());
}
fn folded_into_a_string() {
    let r = std::panic::catch_unwind(|| {
        let mut seen = 0;
        vec![D(61), D(62)]
            .into_iter()
            .map(|x| vec![D(x.0 + 10), x])
            .flatten()
            .map(|x| {
                seen += 1;
                if seen == 2 {
                    panic!("consumer");
                }
                char::from(b'a' + (x.0 % 26) as u8)
            })
            .collect::<String>()
    });
    println!("folded into a string {}", r.is_err());
}
fn from_its_back() {
    let sum = vec![D(7), D(8)]
        .into_iter()
        .map(|x| vec![D(x.0 + 10), x])
        .flatten()
        .rfold(0, |sum, d| sum + d.0);
    println!("from its back {sum}");
}
fn main() {
    source_drop_panics();
    folded_into_a_string();
    source_ended();
    by_its_own_fold();
    from_its_back();
    mapped();
    flat_mapped();
    broken_off();
    of_a_range();
}
