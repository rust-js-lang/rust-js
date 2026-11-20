//@ library-differs
// A `Box` or a `&mut` of std's adapter steps as std's wrapper does, however
// generic code is given it: through a trait, `impl Is<I>`, where `I` comes
// with its dictionary (#2185, #2187); a box of a box, a box of a type
// parameter's value, a `&mut` of a box; and a `Box<dyn Iterator>`, called on,
// given straight or through the trait (#2186). Std's sized `Box<I>` leaves
// `fold` and `count` to the traits' defaults over its `next`, its sized
// `&mut I` folds by `I`'s `try_fold`, and a `dyn`'s vtable holds no `fold`,
// so none of them asks the crate's own `fold` under a `map`; a `skip`'s
// `nth` is passed on, so `nth(1)` asks the crate's `nth(2)`; and a `&mut`
// of a `skip` folds by `Skip::try_fold`, which asks the crate's `nth(0)`,
// never by its `next`'s `nth(1)` (#2190).
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        for x in self.by_ref() {
            acc = f(acc, x);
        }
        acc
    }
    fn count(self) -> usize {
        println!("own count");
        self.fold(0, |n, _| n + 1)
    }
}

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn total<I: Iterator<Item = u32>>(it: impl Is<I>) -> u32 {
    let it: I = Is::is(it);
    it.fold(0, |a, x| a * 10 + x)
}

fn counted<I: Iterator<Item = u32>>(it: impl Is<I>) -> usize {
    let it: I = Is::is(it);
    it.count()
}

fn second<I: Iterator<Item = u32>>(it: impl Is<I>) -> Option<u32> {
    let mut it: I = Is::is(it);
    it.nth(1)
}

fn from_back<I: DoubleEndedIterator<Item = u32>>(it: impl Is<I>) -> u32 {
    let it: I = Is::is(it);
    it.rfold(0, |a, x| a * 10 + x)
}

fn direct<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.fold(0, |a, x| a * 10 + x)
}

fn lent<J: Iterator<Item = u32>>(mut j: J) -> u32 {
    total(&mut j)
}

fn boxed<J: Iterator<Item = u32>>(j: J) -> u32 {
    total(Box::new(j))
}

fn main() {
    println!("box {}", total(Box::new(Countdown(3).map(|x| x * 2))));
    println!("box count {}", counted(Box::new(Countdown(3).map(|x| x + 1))));
    println!("box nth {:?}", second(Box::new(Countdown(4).skip(1))));
    println!("box filter {}", total(Box::new(Countdown(3).filter(|x| x % 2 == 0))));
    println!("box of box {}", total(Box::new(Box::new(Countdown(3).map(|x| x * 2)))));
    println!("bare box {}", total(Box::new(Countdown(3))));
    println!(
        "box back {}",
        from_back(Box::new([1u32, 2, 3].into_iter().map(|x| x + 1)))
    );
    println!("box of a parameter {}", boxed(Countdown(3).map(|x| x * 2)));

    let mut adapter = Countdown(3).map(|x| x * 2);
    println!("mut {}", total(&mut adapter));
    let mut held = Box::new(Countdown(3).map(|x| x * 2));
    println!("mut box {}", total(&mut held));
    println!("mut of a parameter {}", lent(Countdown(3).map(|x| x * 2)));
    let mut skipped = Countdown(4).skip(1);
    println!("mut skip {}", total(&mut skipped));

    let it: Box<dyn Iterator<Item = u32>> = Box::new(Countdown(3).map(|x| x * 2));
    println!("dyn {}", total(it));
    let it: Box<dyn Iterator<Item = u32>> = Box::new(Countdown(3).map(|x| x * 2));
    println!("dyn direct {}", direct(it));
    let it: Box<dyn Iterator<Item = u32>> = Box::new(Countdown(3).map(|x| x * 2));
    println!("dyn fold {}", it.fold(0, |a, x| a * 10 + x));
    let it: Box<dyn Iterator<Item = u32>> = Box::new(Countdown(3).map(|x| x * 2));
    println!("dyn count {}", it.count());
    let mut it: Box<dyn Iterator<Item = u32>> = Box::new(Countdown(3).map(|x| x * 2));
    println!("mut dyn {}", total(&mut it));
    let it: Box<dyn Iterator<Item = u32>> = Box::new(Countdown(3));
    println!("bare dyn {}", it.count());
}
