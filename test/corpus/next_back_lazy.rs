// `next_back()` of a filter of an array's items runs its test on the items
// from the back, up to the last it keeps, as Rust's `Filter::next_back`
// does: made knowing its back where it's made, `$filterEnds` (ADRs 0526,
// 0990).
fn main() {
    let v = vec![1, 2, 3];
    let last = v
        .iter()
        .filter(|x| {
            println!("saw {x}");
            **x > 1
        })
        .next_back();
    println!("{last:?}");
    let mut odd = v.iter().filter(|x| **x % 2 == 1);
    println!("{:?} {:?} {:?}", odd.next_back(), odd.next(), odd.next_back());
}
