// `scan` is lazy, as Rust's is: what it's given is stepped only as far as
// it gives, so a closure before it runs no further; of items with a
// destructor, its source closed when it ends; and a `Some(None)` it keeps
// is taken out of its box (ADR 0517).
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn lazy() {
    let v = vec![1, 2, 3, 4];
    let r: Vec<i32> = v
        .iter()
        .map(|x| {
            println!("map {x}");
            x * 2
        })
        .scan(0, |acc, x| {
            *acc += x;
            (*acc < 6).then_some(*acc)
        })
        .collect();
    println!("{r:?}");
}
fn kept() {
    let v = vec![Loud(1), Loud(2), Loud(3), Loud(4)];
    let sums: Vec<u32> = v
        .into_iter()
        .scan(0u32, |acc, l| {
            *acc += u32::from(l.0);
            (*acc < 4).then_some(*acc)
        })
        .collect();
    println!("{sums:?}");
    let kept: Vec<Option<u8>> = [1u8, 0, 2]
        .iter()
        .scan((), |_, &x| Some((x > 0).then_some(x)))
        .collect();
    println!("{kept:?}");
    let arr = [5, 6, 7];
    let eager: Vec<i32> = arr
        .iter()
        .scan(1, |p, &x| {
            *p *= x;
            Some(*p)
        })
        .collect();
    println!("{eager:?}");
}
fn main() {
    lazy();
    kept();
    println!("end");
}
