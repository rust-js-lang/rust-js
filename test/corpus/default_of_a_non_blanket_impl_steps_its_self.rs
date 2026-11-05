// A trait's default copied into an impl that isn't a blanket one, `impl<I:
// Iterator> Keeping for Wrapper<I>`: its `Self` is the impl's `Wrapper<I>`,
// stepped by its own `next`, as the impl's terms say, where its body read
// it as a JS iterator, a type parameter's, as its trait's terms do (#803).

struct D(u64);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Held<I> {
    iter: I,
}

impl<I> Drop for Held<I> {
    fn drop(&mut self) {
        println!("drop held");
    }
}

struct Wrapper<I>(I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

// The crate's own iterator, of items with a destructor.
struct Countdown(u64);

impl Iterator for Countdown {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        self.0 -= 1;
        (self.0 > 17).then(|| D(self.0))
    }
}

trait Keeping: Iterator {
    fn looped(self) -> usize
    where
        Self: Sized,
    {
        let mut n = 0;
        for _ in self {
            n += 1;
        }
        n
    }

    fn collected(self) -> usize
    where
        Self: Sized,
    {
        let all: Vec<_> = self.map(Some).collect();
        all.len()
    }

    fn counted(self) -> usize
    where
        Self: Sized,
    {
        self.count()
    }

    fn stepped(&mut self) -> Option<Self::Item> {
        self.next();
        self.next()
    }

    fn held(self) -> usize
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        let all: Vec<_> = self.map(|i| Held { iter: i.into_iter() }).collect();
        all.len()
    }
}

impl<I: Iterator> Keeping for Wrapper<I> {}

// And one of two parameters, its second one the default doesn't name.
struct Paired<I, U>(I, U);

impl<I: Iterator, U> Iterator for Paired<I, U> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

impl<I: Iterator, U> Keeping for Paired<I, U> {}

// Through generic code too.
fn collected_through<K: Keeping>(keeping: K) -> usize {
    keeping.collected()
}

fn main() {
    println!("{}", Wrapper(Countdown(20)).looped());
    println!("{}", Wrapper(Countdown(20)).collected());
    println!("{}", Wrapper(vec![D(1), D(2)].into_iter()).collected());
    println!("{}", Wrapper(vec![D(3), D(4), D(5)].into_iter()).counted());
    let mut stepping = Wrapper(Countdown(21));
    println!("{:?}", stepping.stepped().map(|d| d.0));
    println!("{:?}", stepping.next().map(|d| d.0));
    println!("{}", Wrapper(vec![Some(D(6)), None].into_iter()).held());
    println!("{}", Paired(vec![D(7)].into_iter(), vec![D(8)]).counted());
    println!("{}", Paired(Countdown(19), D(9)).collected());
    println!("{}", collected_through(Wrapper(Countdown(19))));
    println!("end");
}
