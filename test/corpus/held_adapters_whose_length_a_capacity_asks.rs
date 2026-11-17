// A variable that holds an adapter made in place, whose length, or whose
// parts' lengths, the growth of a `Vec` by what it's made into asks, in a
// crate that reads a `Vec`'s capacity: made knowing it where it's made, as
// one made in place where it's grown is (#1795, #1744). A held map of a
// slice's items grown by an `inspect` of it; a held chain of an `inspect`
// and an array's items; a held map a chain of an `inspect` of which it's
// grown by; one stepped from its back too; one whose items have a
// destructor; one whose length nothing asks, as it was; and of two
// variables of one adapter type, only the one the growth is made of.

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud {}", self.0);
    }
}

fn doubled(v: &[u32]) -> (usize, usize) {
    let d = v.iter().map(|x| x * 2);
    let w: Vec<u32> = d.inspect(|_| ()).collect();
    (w.len(), w.capacity())
}

fn chained(items: Vec<u32>) -> (Vec<u32>, usize, u32) {
    let mut seen = 0;
    let held = items.into_iter().inspect(|_| seen += 1).chain([9, 8]);
    let grown: Vec<u32> = held.collect();
    let capacity = grown.capacity();
    (grown, capacity, seen)
}

fn part(v: &[u32]) -> (Vec<u32>, usize) {
    let mapped = v.iter().map(|x| x + 10);
    let grown: Vec<u32> = mapped.inspect(|_| ()).chain([1, 2]).collect();
    let capacity = grown.capacity();
    (grown, capacity)
}

fn both_ends(v: &[u32]) -> (Option<u32>, Vec<u32>, usize) {
    let mut d = v.iter().map(|x| x + 1);
    let last = d.next_back();
    let rest: Vec<u32> = d.inspect(|_| ()).collect();
    let capacity = rest.capacity();
    (last, rest, capacity)
}

fn loud(n: u32) -> (Vec<String>, usize) {
    let louds: Vec<Loud> = (0..n).map(Loud).collect();
    let held = louds.into_iter().map(|l| format!("{}0", l.0));
    let tens: Vec<String> = held.inspect(|_| ()).collect();
    let capacity = tens.capacity();
    (tens, capacity)
}

fn double(x: &u32) -> u32 {
    x * 2
}

fn one_of_two(v: &[u32]) -> (usize, u32) {
    let grown = v.iter().map(double);
    let summed = v.iter().map(double);
    let w: Vec<u32> = grown.inspect(|_| ()).collect();
    (w.capacity(), summed.sum())
}

fn unasked(v: &[u32]) -> u32 {
    let d = v.iter().map(|x| x * 3);
    d.sum()
}

fn main() {
    for v in [&[1u32, 2, 3, 4, 5][..], &[], &[7; 9]] {
        println!("{:?}", doubled(v));
        println!("{:?}", both_ends(v));
        println!("{:?}", part(v));
        println!("{}", unasked(v));
        println!("{:?}", one_of_two(v));
    }
    println!("{:?}", chained(vec![1, 2, 3]));
    println!("{:?}", chained(Vec::new()));
    println!("{:?}", loud(3));
    println!("{:?}", loud(6));
}
