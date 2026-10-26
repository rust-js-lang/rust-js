// A `&mut` of an iterator, `by_ref()`'s, held by std's adapter, steps by
// what std's impl for `&mut I` passes on to its `I`: its `next`,
// `size_hint` and `nth`, not its `fold`, which is `I`'s `try_fold` (ADR
// 0912). Of a range, of the crate's iterator, and of a type parameter's;
// what's left goes on after.
struct Count {
    at: u32,
    end: u32,
}

impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.at < self.end).then(|| {
            self.at += 1;
            self.at
        })
    }
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("nth {n}");
        self.at += n as u32;
        self.next()
    }
    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

fn halves<I: Iterator<Item = u32>>(mut it: I) -> (u32, Option<u32>) {
    let mut fused = it.by_ref().map(|x| x / 2).fuse();
    let first = fused.nth(1).unwrap_or(0);
    let total = fused.sum::<u32>();
    (first + total, it.next())
}

fn main() {
    let mut numbers = 1..10;
    let mut doubled = numbers.by_ref().map(|x| x * 2);
    println!("{:?} {:?}", doubled.next(), doubled.size_hint());
    println!("{:?}", doubled.nth(2));
    println!("{numbers:?}");

    let mut counted = Count { at: 0, end: 9 };
    let mut fused = counted.by_ref().fuse();
    println!("{:?}", fused.nth(1));
    println!("{}", fused.sum::<u32>());
    println!("{:?}", counted.next());

    let mut again = Count { at: 0, end: 6 };
    let mut peeked = again.by_ref().peekable();
    let first = peeked.peek().copied();
    println!("{first:?} {}", peeked.count());

    println!("{:?}", halves(Count { at: 0, end: 7 }));
    println!("{:?}", halves(3..9));
}
