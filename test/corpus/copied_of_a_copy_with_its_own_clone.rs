// A `Copy` type's copy is its bits: `copied()`, an `Option`'s too,
// `copy_from_slice`, `partition`'s push of a `&T`, an array made of a
// slice and `repeat`'s copies past its first never call a hand-written
// `Clone` of it, where `cloned()`, an `Option`'s too, `to_vec()`,
// `extend_from_slice` and `repeat`'s first copy do, and so does the clone
// of a tuple, an array, an `Option` or a generic derive of it, as std's
// do; a `#[derive(Clone, Copy)]` of a type of no type parameters is its
// bits. Each copy changed apart after, so it's a copy and not the item.
#[derive(Copy, Debug, PartialEq)]
struct P(u8);

impl Clone for P {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Self(self.0 + 100)
    }
}

#[derive(Clone, Copy, Debug)]
struct Wrapped<T>(T);

#[derive(Clone, Copy, Debug)]
struct Plain(P);

// A clone, through generic code's `Clone`, as clippy would have a `Copy`
// value's written as its copy.
fn cloned<T: Clone>(value: &T) -> T {
    value.clone()
}

fn copied_in_generic_code<T: Copy>(v: &[T]) -> Vec<T> {
    let mut it = v.iter().copied();
    it.next();
    let again = it.clone();
    it.chain(again).collect()
}

fn main() {
    let ps = [P(1), P(2), P(3)];

    let mut copied: Vec<P> = ps.iter().copied().collect();
    copied[0].0 = 9;
    println!("copied {copied:?} {ps:?}");
    let back: Vec<P> = ps.iter().rev().copied().collect();
    println!("back {back:?}");
    for p in ps.iter().copied() {
        print!("{} ", p.0);
    }
    println!();
    let mut it = ps.iter().copied();
    it.next();
    let later = it.clone();
    println!("cloned iterator {:?} {:?}", it.collect::<Vec<_>>(), later.collect::<Vec<_>>());
    println!("generic {:?}", copied_in_generic_code(&ps));
    println!("option {:?}", ps.first().copied());
    let mut one = P(4);
    println!("option of a &mut {:?}", Some(&mut one).copied());
    println!("option cloned {:?}", ps.last().cloned());

    let mut into = [P(0); 3];
    into.copy_from_slice(&ps);
    into[1].0 = 7;
    println!("copy_from_slice {into:?} {ps:?}");

    println!("cloned {:?}", ps.iter().cloned().collect::<Vec<_>>());
    println!("to_vec {:?}", ps.to_vec());
    let mut extended = Vec::new();
    extended.extend_from_slice(&ps[1..]);
    println!("extend_from_slice {extended:?}");

    let (mut ones, others): (Vec<P>, Vec<P>) = ps.iter().partition(|p| p.0 == 1);
    ones[0].0 = 8;
    println!("partition {ones:?} {others:?}");
    let mut array: [P; 2] = ps[1..].try_into().unwrap();
    array[0].0 = 6;
    println!("array of a slice {array:?}");
    let mut repeated = ps[..2].repeat(3);
    repeated[2].0 = 5;
    println!("repeat {repeated:?}");

    println!("array clone {:?}", cloned(&ps));
    println!("tuple clone {:?}", cloned(&(P(5), 6u8)));
    println!("option clone {:?}", cloned(&Some(P(6))));
    println!("generic derive {:?}", cloned(&Wrapped(P(7))));
    let plain = Plain(P(8));
    let mut copy = cloned(&plain);
    copy.0 .0 = 0;
    println!("derive of no parameters {:?} {:?}", copy, plain);
}
