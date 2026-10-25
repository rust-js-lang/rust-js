// The crate's iterator's own `fold` through `peekable()`, `fuse()` and
// generic code given it as it is, by its dictionary: each calls it as std's
// does, and that `fold`, which takes the iterator, drops it once (#162); a
// `map` of a `peekable()` of one with a destructor drops it.
struct Own(&'static str, u32);

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Iterator for Own {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        println!("next {}", self.0);
        if self.1 < 3 {
            self.1 += 1;
            Some(self.1)
        } else {
            None
        }
    }

    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("fold {}", self.0);
        let mut folded = init;
        while self.1 < 3 {
            self.1 += 1;
            folded = f(folded, self.1);
        }
        folded
    }

    fn nth(&mut self, k: usize) -> Option<u32> {
        println!("nth {} {k}", self.0);
        self.1 += k as u32;
        self.next()
    }
}

struct Plain(&'static str, u32);

impl Drop for Plain {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Iterator for Plain {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.1 < 3 {
            self.1 += 1;
            Some(self.1)
        } else {
            None
        }
    }
}

struct Holder<I> {
    it: I,
}

impl<I: Iterator<Item = u32>> Holder<I> {
    fn total(self) -> u32 {
        self.it.sum()
    }
}

fn main() {
    println!("{}", Own("a", 0).peekable().count());
    let mut peeked = Own("b", 0).peekable();
    println!("{:?}", peeked.peek());
    println!("{}", peeked.sum::<u32>());
    println!("{}", Own("c", 0).fuse().sum::<u32>());
    println!("{}", Own("d", 0).fuse().map(|x| x + 1).sum::<u32>());
    println!("{}", Plain("e", 0).peekable().sum::<u32>());
    println!("{}", Plain("f", 0).fuse().map(|x| x * 2).sum::<u32>());
    println!("{}", Holder { it: Own("g", 0) }.total());
    println!("{}", Holder { it: Plain("h", 0) }.total());
    println!("{}", Own("j", 0).sum::<u32>());
    println!("{}", Own("k", 0).map(|x| x + 1).sum::<u32>());
    let mut peeked = Own("l", 0).peekable();
    peeked.peek();
    println!("{:?}", peeked.map(|x| x * 2).collect::<Vec<_>>());
    println!("{}", Own("m", 0).peekable().map(|x| x * 2).sum::<u32>());
    println!("{}", Own("n", 0).fuse().map(|x| x * 2).sum::<u32>());
    println!("{}", Own("o", 0).peekable().filter(|x| x % 2 == 1).count());
    println!("{}", Plain("p", 0).peekable().map(|x| x * 2).sum::<u32>());
    println!("{:?}", Plain("q", 0).peekable().take(1).collect::<Vec<_>>());
    println!("{:?}", Plain("r", 0).fuse().map(|x| x + 1).collect::<Vec<_>>());
    let mut stepped = Plain("s", 0).peekable();
    stepped.peek();
    println!("{:?}", stepped.map(|x| x * 2).collect::<Vec<_>>());
    // A held map's and inspect's fold, std's: their source's, the crate's
    // own, through a peekable (#264).
    println!("{}", Own("t", 0).map(|x| x * 2).peekable().sum::<u32>());
    println!("{}", Own("u", 0).inspect(|_| ()).peekable().sum::<u32>());
    println!(
        "{}",
        Own("w", 0)
            .skip(1)
            .inspect(|_| ())
            .peekable()
            .map(|x| x + 1)
            .sum::<u32>()
    );
    println!("{}", Plain("v", 0).map(|x| x + 1).peekable().sum::<u32>());
    // A held flat map's fold, std's: its source's, through its `map`, the
    // crate's own, each item's array folded in turn (#297).
    println!("{}", Own("x", 0).flat_map(|x| [x, x * 10]).fuse().sum::<u32>());
    println!("end");
}
