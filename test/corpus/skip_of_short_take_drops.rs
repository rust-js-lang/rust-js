// A skip of a take shorter than it skips, of a source with its own `nth`, a
// compiled `Fuse`'s: std's `Skip::nth`, `count`, `fold`, `try_fold` and
// `last` ask the take's `nth`, which past its end asks its source's
// `nth(n - 1)` and drops what that gives back, by the items' drop the skip
// passes on (#492, nightly fuzz seed 12456541). An `enumerate`'s `nth`
// passes it on too, of the item it counts.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3)]
}

fn main() {
    println!("nth {:?}", ds().into_iter().fuse().take(2).skip(1).nth(2).map(|d| d.0));
    println!("count {}", ds().into_iter().fuse().take(1).skip(3).count());
    println!(
        "fold {}",
        ds().into_iter().fuse().take(1).skip(3).fold(0, |a, d| a + d.0)
    );
    println!("last {:?}", ds().into_iter().fuse().take(1).skip(3).last().map(|d| d.0));
    println!("any {}", ds().into_iter().fuse().take(1).skip(3).any(|d| d.0 == 1));
    println!(
        "enumerate {:?}",
        ds().into_iter()
            .rev()
            .fuse()
            .take(1)
            .enumerate()
            .nth(1)
            .map(|(_, d)| d.0)
    );
    println!(
        "within {:?}",
        ds().into_iter().fuse().take(3).skip(1).nth(0).map(|d| d.0)
    );
}
