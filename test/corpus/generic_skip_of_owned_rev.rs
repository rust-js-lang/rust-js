// Generic code skipping or chaining into what it's given, a `rev()` of a `Vec`'s items
// with a destructor: what's skipped is dropped as std's `Rev` passes over it,
// the `Vec`'s tail as one slice, front to back (#286, #382).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn skipped<I: Iterator<Item = Noisy>>(it: I) -> Vec<u32> {
    it.skip(2).map(|n| n.0).collect()
}

fn nth_of<I: Iterator<Item = Noisy>>(mut it: I) -> Option<u32> {
    it.nth(1).map(|n| n.0)
}

fn chained_nth<I: Iterator<Item = Noisy>>(a: I, b: I) -> Option<u32> {
    a.chain(b).nth(3).map(|n| n.0)
}

fn noisy(n: u32) -> Vec<Noisy> {
    (1..=n).map(Noisy).collect()
}

fn main() {
    println!("{:?}", skipped(noisy(5).into_iter().rev()));
    println!("{:?}", nth_of(noisy(4).into_iter().rev()));
    println!("{:?}", skipped(noisy(4).into_iter()));
    println!(
        "{:?}",
        chained_nth(noisy(5).into_iter().rev(), noisy(2).into_iter().rev())
    );
}
