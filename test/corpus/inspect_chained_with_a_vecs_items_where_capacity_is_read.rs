// A `Vec` grown by a chain of an `inspect()` of a `Vec`'s items and
// another `Vec`'s, in a crate that reads a capacity, which std makes its
// iterator's hint's lower bound, each part's summed. MIR holds the
// `inspect` in a temporary while `vec![..]` makes the chain's argument,
// so it's made knowing its length there, as a variable's is (#2041): by a
// closure, by a function, of items with a destructor, of none, and
// extending a `Vec` that has some.
struct Probe(u32);

impl Drop for Probe {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn see(n: &u32) {
    println!("saw {n}");
}

fn main() {
    let mut seen = 0;
    let chained: Vec<u32> = vec![1u32, 2, 3]
        .into_iter()
        .inspect(|_| seen += 1)
        .chain(vec![9, 8])
        .collect();
    println!("{chained:?} {} {seen}", chained.capacity());

    let by_function: Vec<u32> = vec![4u32].into_iter().inspect(see).chain(vec![7]).collect();
    println!("{by_function:?} {}", by_function.capacity());

    let probes: Vec<Probe> = vec![Probe(1), Probe(2)]
        .into_iter()
        .inspect(|p| println!("see {}", p.0))
        .chain(vec![Probe(9)])
        .collect();
    println!("{} {}", probes.len(), probes.capacity());

    let none: Vec<u32> = Vec::<u32>::new().into_iter().inspect(see).chain(vec![5]).collect();
    println!("{none:?} {}", none.capacity());

    let mut grown = vec![0u32];
    grown.extend(vec![6u32, 5].into_iter().inspect(see).chain(vec![4]));
    println!("{grown:?} {}", grown.capacity());
    drop(probes);
    println!("end");
}
