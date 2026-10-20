// `rev()` of an array's items with a destructor steps them from the back,
// and drops what's left of them front to back, as Rust's `Rev` drops the
// iterator it holds (ADR 0516).
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let v = vec![Loud(1), Loud(2), Loud(3)];
    let mut r = v.into_iter().rev();
    println!("{:?}", r.next().map(|l| l.0));
    drop(r);
    let w = vec![Loud(4), Loud(5), Loud(6)];
    let all: Vec<u8> = w.into_iter().rev().map(|l| l.0).collect();
    println!("{all:?}");
    let x = vec![Loud(7), Loud(8), Loud(9)];
    let firsts: Vec<Loud> = x.into_iter().rev().take(1).collect();
    println!("{}", firsts.len());
    println!("end");
    let mut held = vec![Loud(10), Loud(11), Loud(12), Loud(13)].into_iter();
    held.next();
    let mut back = held.rev();
    println!("{:?}", back.next().map(|l| l.0));
    drop(back);
    println!("end");
}
