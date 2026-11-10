// A clone of an adapter made over what a function gives behind an `impl
// Iterator + Clone`, std's struct it made: its source cloned by its own
// compiled `Clone`, its value's form, never as an adapter the crate clones
// apart (#1241, the user's decision (A) of #1226).
fn doubled(v: &[i32]) -> impl Iterator<Item = i32> + Clone + '_ {
    v.iter().map(|x| x * 2)
}

fn main() {
    let v = [1, 2, 3, 4];
    let big = doubled(&v).filter(|x| *x > 2);
    let again = big.clone();
    println!("{:?} {:?}", big.collect::<Vec<_>>(), again.count());
    let plus = doubled(&v).map(|x| x + 1);
    let copy = plus.clone();
    println!("{:?} {:?}", plus.collect::<Vec<_>>(), copy.last());
    let both = doubled(&v).chain(doubled(&v[..2]));
    let twice = both.clone();
    println!("{} {:?}", both.count(), twice.collect::<Vec<_>>());
}
