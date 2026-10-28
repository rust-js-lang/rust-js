// A `rev()` held in a variable, std's compiled `Rev`, steps its source by
// its `next_back`: a filter, a `filter_map` or a map made where it's made
// knowing its back, of an array's items or a range's (ADR 0871; #382).
fn tens(x: u32) -> u32 {
    x * 10
}
fn main() {
    let v = vec![1u32, 2, 3, 4, 5];
    let mut a = v.iter().filter(|x| **x % 2 == 1).rev();
    println!("{:?} {:?}", a.next(), a.next_back());
    let mut b = v.iter().filter_map(|x| (*x > 2).then_some(x * 2)).rev();
    println!("{:?} {:?}", b.next(), b.next());
    let mut c = (1..6u32).map(tens).rev();
    println!("{:?} {:?}", c.next(), c.next_back());
    let mut d = (1..6u32).filter(|x| x % 2 == 0).rev();
    println!("{:?} {:?}", d.next(), d.next());
}
