// Std's `last` of a `skip`, a `fuse`, a `peekable`, a `copied` and a
// `chain` is its source's, which takes the source whole (#522): what the
// crate's own `last` or `fold` leaves it drops itself, once, and a source
// whose `last` is std's is dropped after it, as the adapter's `last` ends.
// Generic code's `last()`, `max()`, `sum()` and `for_each` of an iterator
// with a destructor drop it after too, unless the crate's own `fold` or
// `last` took it.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Lasts(Vec<D>);

impl Iterator for Lasts {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn last(mut self) -> Option<D> {
        println!("own last");
        self.0.pop()
    }
}

struct Held(Vec<D>, &'static str);

impl Iterator for Held {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        println!("drop held {}", self.1);
    }
}

struct Folds(Vec<u32>, &'static str);

impl Iterator for Folds {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold {}", self.1);
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

impl Drop for Folds {
    fn drop(&mut self) {
        println!("drop folds {}", self.1);
    }
}

struct Refs<'a>(&'a [u32], &'static str);

impl<'a> Iterator for Refs<'a> {
    type Item = &'a u32;

    fn next(&mut self) -> Option<&'a u32> {
        let (first, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(first)
    }

    fn last(self) -> Option<&'a u32> {
        println!("own last {}", self.1);
        self.0.last()
    }
}

impl Drop for Refs<'_> {
    fn drop(&mut self) {
        println!("drop refs {}", self.1);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3)]
}

fn show(what: &str, last: Option<D>) {
    println!("{what} {:?}", last.as_ref().map(|d| d.0));
}

fn end<I: Iterator>(it: I) -> Option<I::Item> {
    it.last()
}

fn top<I: Iterator>(it: I) -> Option<I::Item>
where
    I::Item: Ord,
{
    it.max()
}

fn total<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.sum()
}

fn each<I: Iterator<Item = u32>>(it: I) {
    it.for_each(|x| println!("each {x}"));
}

fn main() {
    show("skip", Lasts(ds()).skip(1).last());
    show("fuse", Lasts(ds()).fuse().last());
    show("peekable", Lasts(ds()).peekable().last());
    let mut peeked = Lasts(ds()).peekable();
    peeked.peek();
    show("peeked", peeked.last());
    show("chain", Lasts(ds()).chain(Lasts(vec![D(5), D(6)])).last());
    show("chain held a", Held(ds(), "a").chain(Lasts(vec![D(5)])).last());
    show("chain held b", Lasts(ds()).chain(Held(vec![D(7)], "b")).last());
    show("chain skip", Lasts(ds()).chain(Lasts(vec![D(5), D(6)])).skip(1).last());
    show("skip held", Held(ds(), "s").skip(1).last());
    show("fuse held", Held(ds(), "f").fuse().last());
    show("peekable held", Held(ds(), "p").peekable().last());
    println!("folds {:?}", Folds(vec![1, 2, 3], "s").skip(1).last());
    println!("folds {:?}", Folds(vec![1, 2], "f").fuse().last());
    let items = [4, 5, 6];
    println!("copied {:?}", Refs(&items, "c").copied().last());
    println!("copied {:?}", Refs(&items, "s").skip(1).copied().last());
    show("generic own", end(Lasts(ds())));
    show("generic held", end(Held(ds(), "g")));
    show("generic skip", end(Lasts(ds()).skip(2)));
    println!("generic folds {:?}", end(Folds(vec![1, 2], "g")));
    println!("max {:?}", top(Folds(vec![2, 9, 4], "m")));
    println!("max {:?}", top(Held(ds(), "m").map(|d| d.0)));
    println!("sum {}", total(Folds(vec![1, 2], "t")));
    each(Folds(vec![3], "e"));
    println!("sum {}", total(Held(ds(), "t").map(|d| d.0)));
}
