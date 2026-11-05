// A `rev()` of a type parameter a struct holds, given as it is (ADR 0411),
// stepped from its back by `nth_back`, `rfold` and `rfind`: `$rev` of the JS
// iterator its dictionaries step, whose back steps are its forward ones,
// `nth_back` its `nth`, the crate's own where it writes one, as std's `Rev`
// passes them on; of a `Vec`'s items with a destructor, what its `nth`
// passes over dropped as one slice (#718).
struct Held<I>(I);
impl<I: DoubleEndedIterator<Item = u32>> Held<I> {
    fn second_last(self) -> Option<u32> {
        self.0.rev().nth_back(1)
    }
    fn folded(self) -> u32 {
        self.0.rev().rfold(0, |n, x| n * 10 + x)
    }
    fn found(self) -> Option<u32> {
        self.0.rev().rfind(|x| x % 2 == 1)
    }
}
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct HeldD<I>(I);
impl<I: DoubleEndedIterator<Item = D>> HeldD<I> {
    fn second_last(self) -> Option<u32> {
        self.0.rev().nth_back(1).map(|d| d.0)
    }
}
struct Count {
    at: u32,
}
impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.at += 1;
        (self.at <= 5).then_some(self.at)
    }
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        self.at += n as u32;
        self.next()
    }
}
impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<u32> {
        None
    }
}
fn main() {
    println!("{:?}", Held(vec![1, 2, 3].into_iter()).second_last());
    println!("{}", Held(vec![1, 2, 3].into_iter()).folded());
    println!("{:?}", Held(vec![1, 2, 3, 4].into_iter()).found());
    println!("{:?}", Held((1..6).map(|x| x * 2)).second_last());
    println!(
        "{:?}",
        HeldD((1..5).map(D).collect::<Vec<_>>().into_iter()).second_last()
    );
    println!("{:?}", Held(Count { at: 0 }).second_last());
    println!("end");
}
