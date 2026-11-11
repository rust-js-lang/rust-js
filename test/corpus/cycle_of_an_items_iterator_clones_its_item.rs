// A `cycle()` of an `Option`'s `into_iter()` or of `once(x)` clones what it
// holds each time it starts again, its item by its own `Clone`, as std's
// `Cycle` clones its iterator: `[x]`'s clone is of its items, as a `Vec`'s
// `into_iter()`'s is. It was the array's copy, its item never cloned, so
// `L(7)` came out three times where Rust's clones give `L(207)`.
#[derive(Debug)]
struct L(u32);

impl Clone for L {
    fn clone(&self) -> L {
        println!("clone {}", self.0);
        L(self.0 + 100)
    }
}

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let some: Vec<L> = Some(L(7)).into_iter().cycle().take(3).collect();
    println!("{some:?}");
    let once: Vec<L> = std::iter::once(L(9)).cycle().take(3).collect();
    println!("{once:?}");
    let mut it = Some(L(3)).into_iter();
    let copy = it.clone();
    println!("{:?} {}", it.next(), copy.count());
}
