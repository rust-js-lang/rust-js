// A std trait's associated type of a type parameter, `I::IntoIter` and
// `I::Item` (ADR 0421): what generic code drops of one is its caller's drop,
// as a type parameter's is, given as `dropIIntoIter`.

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// An iterator with a destructor of its own, as smallvec's `Drain` is.
struct Drain {
    rest: std::vec::IntoIter<Loud>,
}

impl Iterator for Drain {
    type Item = Loud;
    fn next(&mut self) -> Option<Loud> {
        self.rest.next()
    }
}

impl Drop for Drain {
    fn drop(&mut self) {
        println!("drain dropped, {} left", self.rest.len());
    }
}

/// Its first, and the iterator dropped, with what it hasn't given.
fn first<I: IntoIterator>(items: I) -> Option<I::Item> {
    let mut it = items.into_iter();
    it.next()
}

/// Its own iterator given on, as smallvec's `insert_many` gives `extend`.
fn relay<I: IntoIterator>(items: I) -> Option<I::Item> {
    let it = items.into_iter();
    first(it)
}

/// Its first item dropped, and the iterator given back.
fn skip_one<I: Iterator>(mut it: I) -> I {
    it.next();
    it
}

struct Bag(Vec<Loud>);

impl Extend<Loud> for Bag {
    fn extend<I: IntoIterator<Item = Loud>>(&mut self, items: I) {
        let mut it = items.into_iter();
        if let Some(item) = it.next() {
            self.0.push(item);
        }
        for item in it {
            if item.0 == "stop" {
                return;
            }
            self.0.push(item);
        }
    }
}

fn main() {
    let found = first(vec![Loud("a"), Loud("b"), Loud("c")]);
    println!("first {}", found.as_ref().map_or("none", |l| l.0));
    let found = relay(Drain { rest: vec![Loud("d"), Loud("e"), Loud("f")].into_iter() });
    println!("relayed {}", found.as_ref().map_or("none", |l| l.0));
    let rest = skip_one(vec![Loud("g"), Loud("h")].into_iter());
    println!("{} left", rest.len());
    drop(rest);
    let mut bag = Bag(Vec::new());
    bag.extend(Drain { rest: vec![Loud("i"), Loud("j"), Loud("stop"), Loud("k")].into_iter() });
    println!("bag {}", bag.0.len());
    bag.extend(vec![Loud("l"), Loud("stop"), Loud("m")]);
    println!("bag {}", bag.0.len());
}
