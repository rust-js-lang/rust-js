// In a crate that reads a `Vec`'s capacity, a `Vec` grown by a zip that's
// exact-sized but not `TrustedLen`, a zip of an `inspect`, is given knowing
// its length, which std grows it by (#1394): where MIR holds it in a
// temporary, its items having a destructor, its maker makes it `$zipEnds`,
// the one form that knows its length, and its `inspect` side `$mapEnds`,
// which keeps its source's, as a map's does. It was refused, "can't count".
#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds(r: std::ops::Range<u32>) -> Vec<D> {
    r.map(D).collect()
}

fn main() {
    let pairs: Vec<(D, D)> = ds(220..223)
        .into_iter()
        .inspect(|d| println!("saw {}", d.0))
        .zip(ds(230..234))
        .collect();
    println!("pairs {} {}", pairs.len(), pairs.capacity());
    let mut grown: Vec<(u32, D)> = Vec::new();
    grown.extend(
        vec![1u32, 2]
            .into_iter()
            .inspect(|n| println!("n {n}"))
            .zip(ds(240..243)),
    );
    println!("grown {} {}", grown.len(), grown.capacity());
}
