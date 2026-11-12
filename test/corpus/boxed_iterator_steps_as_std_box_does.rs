//@ library-differs
// A `Box` of the crate's iterator passes on to it only what std's
// `impl Iterator for Box<I>` writes, `next`, `size_hint`, `nth` and `last`,
// and `next_back` and `nth_back` (alloc/src/boxed/iter.rs): its `fold`,
// `count`, `sum` and `max` are the trait's defaults, stepped by `next`,
// never the iterator's own of them (#1601). Of generic code's `Box<I>`
// too, given the box of it as a JS iterator stepped so; and of one an
// `impl Is<Box<I>>` gives, its `I` held as it is, by its dictionary's
// steps a `Box` passes on alone (#1611).
struct Up(u32, u32);

impl Iterator for Up {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("own size_hint");
        let left = (self.1 - self.0) as usize;
        (left, Some(left))
    }

    fn count(self) -> usize {
        println!("own count");
        (self.1 - self.0) as usize
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
        println!("own nth");
        self.0 += n as u32;
        self.next()
    }

    fn last(self) -> Option<u32> {
        println!("own last");
        (self.0 < self.1).then_some(self.1)
    }
}

fn boxed_count<I: Iterator<Item = u32>>(b: Box<I>) -> usize {
    b.count()
}

fn boxed_last<I: Iterator<Item = u32>>(b: Box<I>) -> Option<u32> {
    b.last()
}

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn counted_through<I: Iterator<Item = u32>>(b: impl Is<Box<I>>) -> usize {
    b.is().count()
}

fn last_through<I: Iterator<Item = u32>>(b: impl Is<Box<I>>) -> Option<u32> {
    b.is().last()
}

fn main() {
    println!("{}", Box::new(Up(0, 3)).sum::<u32>());
    println!("{}", Box::new(Up(0, 3)).count());
    println!("{}", Box::new(Up(0, 3)).fold(0, |a, b| a + b));
    println!("{:?}", Box::new(Up(0, 3)).max());
    println!("{:?}", Box::new(Up(0, 3)).last());
    println!("{:?}", Box::new(Up(0, 5)).nth(1));
    println!("{:?}", Box::new(Up(0, 3)).size_hint());
    println!("{}", boxed_count(Box::new(Up(0, 3))));
    println!("{:?}", boxed_last(Box::new(Up(0, 3))));
    println!("{}", counted_through(Box::new(Up(0, 3))));
    println!("{:?}", last_through(Box::new(Up(0, 3))));
    let mut b = Box::new(Up(0, 4));
    println!("{:?} {:?}", b.next(), b.nth(1));
    println!("{}", b.by_ref().count());
    let d: Box<dyn Iterator<Item = u32>> = Box::new(Up(0, 3));
    println!("{}", d.sum::<u32>());
}
