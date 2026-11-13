// A blanket impl's own method that holds each of its items' iterator,
// where its where clause says `Self::Item: IntoIterator`: `Self` is the
// impl's `I`, so the clause names `<I as Iterator>::Item`, a slot of the
// impl's, and the impl's dictionary is given its `IntoIter`'s drop, as no
// caller through it knows it, so each `Held` drops its field's iterator too
// (#1080; impl_method_drops_its_items_iterator.rs's impl for a `Wrapper<I>`).
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

impl<I: Iterator> Keeping for I {
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
    println!("{}", Countdown(8).held());
    println!("{}", held_through(Countdown(7)));
    println!("{}", Wrapper(Countdown(9)).held());
}
