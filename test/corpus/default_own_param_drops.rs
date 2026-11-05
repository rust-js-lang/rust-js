// A trait's default copied into a blanket impl, of a type parameter of its
// own: its caller gives that parameter's drop, and its associated types',
// after the default's own dictionaries, as the trait declares them (ADR
// 0163), and the default drops what it's given with them (#802).

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

trait Joining: Iterator {
    fn plain<U>(self, extra: U) -> usize
    where
        Self: Sized,
    {
        drop(extra);
        self.count()
    }

    fn counted_with<U: IntoIterator>(self, extra: U) -> usize
    where
        Self: Sized,
    {
        self.count() + extra.into_iter().count()
    }

    fn held_with<U: IntoIterator>(self, extra: U) -> usize
    where
        Self: Sized,
    {
        let held = Held {
            iter: extra.into_iter(),
        };
        let n = self.count();
        drop(held);
        n
    }

    // Kept past the default's end, dropped by its caller.
    fn kept_with<U>(self, extra: U) -> (usize, U)
    where
        Self: Sized,
    {
        (self.count(), extra)
    }
}

impl<T: Iterator> Joining for T {}

// Through generic code, which gives the default's drop as its caller gave it.
fn joined<U>(extra: U) -> usize {
    (0..1).plain(extra)
}

fn main() {
    println!("{}", (0..2).plain(D(1)));
    println!("{}", (0..2).counted_with(vec![D(2), D(3)]));
    println!("{}", (0..2).held_with(vec![D(4)]));
    let (n, kept) = (0..2).kept_with(D(5));
    println!("{n} {}", kept.0);
    drop(kept);
    println!("{}", joined(D(6)));
    println!("{}", (0..2).plain(7u8));
    println!("end");
}
