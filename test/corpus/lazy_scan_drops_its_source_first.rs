// A `scan()` made in place, stepped short and dropped, drops what it holds
// as Rust's `Scan { iter, f, state }` does: its source first, what's left
// of the items, then its state (#760).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let v = vec![D(1), D(2), D(3)];
    let n = v.into_iter().scan(D(10), |s, x| { s.0 += x.0; Some(x.0) }).take(1).count();
    println!("n {n}");
    let w = vec![D(4), D(5)];
    let firsts: Vec<u32> = w.into_iter().scan(D(20), |s, x| { s.0 += 1; if x.0 > 4 { None } else { Some(x.0) } }).collect();
    println!("firsts {firsts:?}");
}
