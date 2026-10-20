// A trait's default that steps its `Self`, an `Iterator`, copied into an
// impl that keeps it: of a blanket impl, `impl<T: Iterator> Trait for T`,
// its `T` is its caller's own value, a range or the crate's own iterator,
// stepped by the impl's `Iterator` dictionary, as the impl's own methods
// are (ADR 0411); of the crate's iterator, `Holder<I>`, its own `next`,
// given `I`'s dictionary, and its items dropped by their given drop (ADR
// 0492).
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Gen(u8);

impl Iterator for Gen {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.0 += 1;
        if self.0 < 5 { Some(self.0) } else { None }
    }
}

struct Louds(u8);

impl Iterator for Louds {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        if self.0 < 5 { Some(Loud(self.0)) } else { None }
    }
}

trait Skipping: Iterator {
    fn skip_one(&mut self) {
        self.next();
    }

    fn skip_two(&mut self) {
        self.skip_one();
        self.skip_one();
    }

    fn second(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.next();
        self.next()
    }
}

impl<T: Iterator> Skipping for T {}

trait Totals: Iterator<Item = u8> {
    fn total(self) -> u8
    where
        Self: Sized,
    {
        self.sum()
    }

    fn doubled(self) -> Vec<u8>
    where
        Self: Sized,
    {
        self.map(|x| x * 2).collect()
    }

    fn looped(self) -> u8
    where
        Self: Sized,
    {
        let mut n = 0;
        for x in self {
            n += x;
        }
        n
    }

    fn first_two(&mut self) -> Vec<u8>
    where
        Self: Sized,
    {
        self.by_ref().take(2).collect()
    }
}

impl<T: Iterator<Item = u8>> Totals for T {}

struct Holder<I>(I);

impl<I: Iterator> Iterator for Holder<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

trait Peeled: Iterator {
    fn drop_first(&mut self) {
        self.next();
    }

    fn drop_two(&mut self) {
        self.drop_first();
        self.drop_first();
    }
}

impl<I: Iterator> Peeled for Holder<I> {}

trait Named {
    fn name(&self) -> &'static str {
        "holder"
    }
}

impl<I: Iterator> Named for Holder<I> {}

fn main() {
    let mut g = Gen(0);
    g.skip_one();
    println!("{:?}", g.next());
    println!("{:?}", Gen(0).second());
    let mut r = 0..4;
    r.skip_two();
    println!("{r:?}");
    let mut v = vec![1, 2, 3].into_iter();
    v.skip_one();
    println!("{:?}", v.next());
    let mut louds = Louds(0);
    louds.skip_two();
    println!("louds skipped");

    println!("{}", Gen(0).total());
    println!("{:?}", Gen(1).doubled());
    println!("{}", (1..4).looped());
    let mut h = Gen(0);
    println!("{:?} {:?}", h.first_two(), h.next());
    let mut range = 0..5;
    println!("{:?} {range:?}", range.first_two());

    let mut held = Holder(Louds(0));
    held.drop_two();
    println!("held {:?}", held.next().map(|l| l.0));
    let mut numbers = Holder(1..4);
    numbers.drop_first();
    println!("{:?} {}", numbers.0, numbers.name());
}
