// A trait's default of a type parameter of its own, `U`, copied into an
// impl of more parameters than its trait, whose second, `U` too, is
// numbered as the default's: each drops with its own drop, the default's
// its caller gives, the impl's its dictionary's (#804).

struct D(u64);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Held<T> {
    held: T,
}

impl<T> Drop for Held<T> {
    fn drop(&mut self) {
        println!("drop held");
    }
}

struct Paired<I, U>(I, U);

impl<I: Iterator, U> Iterator for Paired<I, U> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

trait Counting: Iterator {
    fn counted_with<U: IntoIterator>(self, extra: U) -> usize
    where
        Self: Sized,
    {
        drop(self);
        extra.into_iter().count()
    }

    fn dropped_with<U>(self, extra: U) -> usize
    where
        Self: Sized,
    {
        drop(extra);
        println!("dropped extra");
        drop(self);
        0
    }

    // Held by a type with a destructor of its own, whose drop's evidence is
    // asked of the impl's drops.
    fn held_with<U>(self, extra: U) -> usize
    where
        Self: Sized,
    {
        let held = Held { held: extra };
        drop(held);
        drop(self);
        1
    }

    // Collected, in place, into what holds each.
    fn collected_with<U>(self, extra: Vec<U>) -> usize
    where
        Self: Sized,
    {
        drop(self);
        let all: Vec<Held<U>> = extra.into_iter().map(|held| Held { held }).collect();
        all.len()
    }
}

impl<I: Iterator, U> Counting for Paired<I, U> {}

fn main() {
    println!("{}", Paired(vec![D(1)].into_iter(), vec![D(2)]).counted_with(vec![3u8]));
    println!("{}", Paired(vec![D(4)].into_iter(), 5u8).counted_with(vec![D(6), D(7)]));
    println!("{}", Paired(vec![D(8)].into_iter(), D(9)).dropped_with(D(10)));
    println!("{}", Paired(vec![D(11)].into_iter(), vec![D(12)]).dropped_with(13u8));
    println!("{}", Paired(vec![D(14)].into_iter(), vec![D(15)]).held_with(D(16)));
    println!("{}", Paired(vec![D(17)].into_iter(), 18u8).held_with(19u8));
    println!(
        "{}",
        Paired(vec![D(20)].into_iter(), D(21)).collected_with(vec![D(22), D(23)])
    );
    println!("end");
}
