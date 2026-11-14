// A `Vec` extended by a `TrustedLen` iterator is stepped by its fold, as
// std's `extend_trusted` steps it: a zip of a reversed `Vec`'s items with a
// shorter range takes three and leaves the rest, which drop front to back
// with the iterator. Stepped by `next`, the zip takes one more of its
// longer side, which it drops at once, `drop 2` before `drop 1` (#1803).
// So does a reversed `take`, and a zip of a map.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn main() {
    let mut zipped = Vec::new();
    zipped.extend(ds().into_iter().rev().zip(10..13u32));
    println!("zipped {}", zipped.len());
    let mut taken = Vec::new();
    taken.extend(ds().into_iter().take(3).rev());
    println!("taken {}", taken.len());
    let mut mapped = Vec::new();
    mapped.extend(ds().into_iter().map(|d| D(d.0 + 100)).zip(10..12u32));
    println!("mapped {}", mapped.len());
}
