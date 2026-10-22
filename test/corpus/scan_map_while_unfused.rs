// `scan` and `map_while` aren't fused, as Rust's aren't: a `None` ends
// what's stepped now, and stepping on after it steps the source on.
fn main() {
    let v = [1, 2, 0, 3, 4];
    let mut sums = v.iter().scan(0, |sum, &x| {
        if x == 0 {
            None
        } else {
            *sum += x;
            Some(*sum)
        }
    });
    let first: Vec<i32> = sums.by_ref().collect();
    let second: Vec<i32> = sums.collect();
    println!("{first:?} {second:?}");

    let w = [1, -1, 2, 3];
    let mut fits = w.iter().map_while(|&x| u32::try_from(x).ok());
    println!("{:?} {:?} {:?}", fits.next(), fits.next(), fits.next());
}
