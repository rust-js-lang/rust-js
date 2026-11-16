// Two `impl Trait`s of one trait method's signature: each named in its
// dictionary by its method's word and its 0-based position, `Pair0` and
// `Pair1`, so it carries each one's drop and facts, where one word would
// name both (#1320, ADR 0178, amended). A method with one keeps its word,
// `One`. Generic code holding each part drops it as Rust does, in order.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Pair {
    fn both() -> (impl Sized, impl Sized);
    fn parts(&self) -> (impl Iterator<Item = u32>, impl Sized);
    fn one(&self) -> impl Iterator<Item = u32>;
}

struct Louds;

impl Pair for Louds {
    fn both() -> (impl Sized, impl Sized) {
        (Loud("a"), Loud("b"))
    }

    fn parts(&self) -> (impl Iterator<Item = u32>, impl Sized) {
        (vec![1, 2, 3].into_iter().map(|x| x * 10), Loud("held"))
    }

    fn one(&self) -> impl Iterator<Item = u32> {
        [4, 5].into_iter()
    }
}

struct Quiet;

impl Pair for Quiet {
    fn both() -> (impl Sized, impl Sized) {
        (1u8, "two")
    }

    fn parts(&self) -> (impl Iterator<Item = u32>, impl Sized) {
        (0..2, ())
    }

    fn one(&self) -> impl Iterator<Item = u32> {
        std::iter::empty()
    }
}

fn held<T: Pair>() {
    let _kept = T::both();
    println!("holding");
}

fn first_dropped<T: Pair>() {
    let (a, b) = T::both();
    drop(b);
    println!("second dropped first");
    drop(a);
}

fn summed<T: Pair>(t: &T) -> u32 {
    let (it, held) = t.parts();
    let n: u32 = it.sum();
    println!("summed {n}");
    drop(held);
    n + t.one().sum::<u32>()
}

fn main() {
    held::<Louds>();
    first_dropped::<Louds>();
    println!("{}", summed(&Louds));
    held::<Quiet>();
    println!("{}", summed(&Quiet));
}
