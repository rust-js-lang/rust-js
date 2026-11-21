// `array::from_fn(f)` and `Vec::from_fn(n, f)`, Rust 1.100's: `f` given
// each index; of items with a destructor, what's made dropped in order
// where `f` panics part-way, as std's guard drops it (#2278), and a
// `Vec`'s room asked first; of none, `Array.from` (#1656).
struct L(usize);
impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn squares(n: usize) -> Vec<u32> {
    Vec::from_fn(n, |i| (i * i) as u32)
}
fn idx(i: usize) -> String {
    format!("#{i}")
}
fn main() {
    let a: [u32; 3] = std::array::from_fn(|i| (i * 10) as u32);
    println!("{a:?} {:?} {:?}", squares(4), Vec::from_fn(3, idx));
    let made: Vec<L> = Vec::from_fn(2, L);
    println!("{}", made.len());
    drop(made);
    let r = std::panic::catch_unwind(|| {
        let v: [L; 4] = std::array::from_fn(|i| if i == 2 { panic!("at {i}") } else { L(i) });
        v.len()
    });
    println!("{}", r.is_err());
    let r = std::panic::catch_unwind(|| {
        let v: Vec<L> = Vec::from_fn(5, |i| if i == 3 { panic!("at {i}") } else { L(10 + i) });
        v.len()
    });
    println!("{}", r.is_err());
    let e: Vec<u8> = Vec::from_fn(0, |_| unreachable!());
    println!("{e:?} {}", e.capacity());
}
