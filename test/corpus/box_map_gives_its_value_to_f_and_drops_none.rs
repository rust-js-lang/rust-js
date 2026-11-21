// `Box::map(b, f)`, Rust 1.100's: `f` of what the box holds, by move, so
// `map` drops none of it: `f` drops it, keeps it in what it gives, or
// drops it as it panics; of a fn item and in generic code, as native's (#1656).
struct L(u32);
impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn twice(n: u32) -> u64 {
    u64::from(n) * 2
}
fn boxed_len<T>(b: Box<Vec<T>>) -> Box<usize> {
    Box::map(b, |v| v.len())
}
fn main() {
    let a = Box::new(L(1));
    let b = Box::map(a, |l| L(l.0 + 10));
    println!("mapped {}", b.0);
    let kept = Box::map(Box::new(L(2)), |l| (l, 5u8));
    println!("kept {} {}", kept.0.0, kept.1);
    let v = Box::new(vec![1, 2, 3]);
    let w = Box::map(v, |mut v| {
        v.push(4);
        v
    });
    println!("{w:?}");
    let t = Box::map(Box::new(21u32), twice);
    println!("{t}");
    println!("{}", boxed_len(Box::new(vec!["a", "b"])));
    let s = Box::map(Box::new(String::from("hi")), |s| s + "!");
    println!("{s}");
    let r = std::panic::catch_unwind(|| {
        Box::map(Box::new(L(3)), |l| -> u32 {
            if l.0 == 3 { panic!("boom") } else { l.0 }
        })
    });
    println!("{}", r.is_err());
    println!("end");
}
