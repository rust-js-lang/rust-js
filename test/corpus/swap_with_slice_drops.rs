// `swap_with_slice` of items with a destructor swaps them, dropping none,
// and each is dropped once, where its new owner drops it.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut a = vec![Loud(1), Loud(2)];
    let mut b = vec![Loud(3), Loud(4)];
    a.swap_with_slice(&mut b);
    println!("{} {}", a[0].0, b[1].0);
    drop(a);
    println!("between");
}
