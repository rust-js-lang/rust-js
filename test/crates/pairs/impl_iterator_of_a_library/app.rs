//! What a library's `impl DoubleEndedIterator` hides, seen from the app:
//! counted, enumerated and stepped from its back as the type it is, each
//! item dropped where Rust drops it (#1142).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds(from: u32) -> Vec<D> {
    (from..from + 3).map(D).collect()
}

pub fn main() {
    println!("{}", dep::peeked(ds(1)).enumerate().count());
    let back: Vec<(usize, u32)> = dep::mapped(ds(10), |d| D(d.0 + 100))
        .enumerate()
        .rev()
        .map(|(i, d)| (i, d.0))
        .collect();
    println!("{back:?}");
    println!("{}", dep::queue(ds(20)).enumerate().count());
    let rev: Vec<u32> = dep::queue(ds(30)).rev().map(|d| d.0).collect();
    println!("{rev:?}");
    let mut held = dep::queue(ds(40));
    println!("{:?} {}", held.next_back().map(|d| d.0), held.len());
}
