// A trait's associated type bound by `Iterator` that an impl gives as the
// crate's own iterator, with a destructor: generic code has its value as the
// JS iterator the impl's dictionary makes of it, knowing its back where the
// trait bounds it so, and drops it by the dictionary's drop of what it was
// made of, after a consumer that doesn't close it, as one that does closes
// it, and where a function it's given writes over it (#978).
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        println!("from the back");
        self.next().map(|x| x + 10)
    }
}

impl Drop for Countdown {
    fn drop(&mut self) {
        println!("drop at {}", self.0);
    }
}

trait Source {
    type Iter: DoubleEndedIterator<Item = u32>;
    fn items(&self) -> Self::Iter;
}

struct Counted(u32);

impl Source for Counted {
    type Iter = Countdown;
    fn items(&self) -> Countdown {
        Countdown(self.0)
    }
}

fn first<S: Source>(source: &S) -> Option<u32> {
    let mut items = source.items();
    items.next()
}

fn last<S: Source>(source: &S) -> Option<u32> {
    source.items().next_back()
}

fn all<S: Source>(source: &S) -> Vec<u32> {
    source.items().collect()
}

fn backwards<S: Source>(source: &S) -> Vec<u32> {
    source.items().rev().collect()
}

fn put<I: Iterator>(it: &mut I, other: I) {
    *it = other;
}

fn replaced<S: Source>(source: &S, other: &S) -> Option<u32> {
    let mut items = source.items();
    put(&mut items, other.items());
    items.next()
}

fn main() {
    println!("{:?}", first(&Counted(3)));
    println!("{:?}", last(&Counted(3)));
    println!("{:?}", all(&Counted(2)));
    println!("{:?}", backwards(&Counted(2)));
    println!("{:?}", replaced(&Counted(2), &Counted(5)));
}
