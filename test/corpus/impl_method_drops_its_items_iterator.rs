// An impl's own method that holds each of its items' iterator, where its
// where clause says `Self::Item: IntoIterator`: the iterator's drop is a
// slot of the method's, `<I as Iterator>::Item`'s `IntoIter`, as the bounds
// here say `Self::Item` is, given by its caller as a generic function's is,
// so each `Held` drops its field's iterator too (#1080).
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

/// Makes each item, holding nothing to drop itself.
struct Countdown(u64);

impl Iterator for Countdown {
    type Item = Option<D>;

    fn next(&mut self) -> Option<Option<D>> {
        self.0 -= 1;
        (self.0 > 4).then(|| (self.0 % 2 == 0).then(|| D(self.0)))
    }
}

struct Wrapper<I>(I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

trait Keeping: Iterator {
    fn held(self) -> usize
    where
        Self: Sized,
        Self::Item: IntoIterator;
}

impl<I: Iterator> Keeping for Wrapper<I> {
    fn held(self) -> usize
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        let all: Vec<_> = self.map(|i| Held { iter: i.into_iter() }).collect();
        all.len()
    }
}

fn held_through<K: Keeping>(keeping: K) -> usize
where
    K::Item: IntoIterator,
{
    keeping.held()
}

fn main() {
    println!("{}", Wrapper(Countdown(8)).held());
    println!("{}", held_through(Wrapper(Countdown(7))));
}
