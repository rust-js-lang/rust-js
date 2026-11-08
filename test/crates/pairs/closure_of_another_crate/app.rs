//! A library's iterator, of this crate's items with a destructor: the
//! library's closure that took the last item drops it, where Rust does.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let pairs: Vec<(D, D)> = dep::Paired { it: vec![D(1), D(2), D(3)].into_iter() }.collect();
    println!("pairs {}", pairs.len());
    for (a, b) in &pairs {
        println!("{} {}", a.0, b.0);
    }
}
