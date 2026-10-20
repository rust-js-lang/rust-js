// `flat_map` and `flatten` of what has a destructor (ADR 0510): each inner
// iterator closed to drop what it hasn't given, and the source to drop what
// it holds, the source first, as Rust drops a `FlatMap`; and `flat_map` of a
// function that gives an `Option`, its `Some`s.
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn inner_left() {
    let first: Vec<Loud> = (0..3).flat_map(|i| vec![Loud(i), Loud(i + 10)]).take(1).collect();
    println!("inner left {}", first.len());
    let counts = [1u8, 2];
    let made: Vec<Loud> = counts.iter().flat_map(|&n| (0..n).map(Loud)).take(1).collect();
    println!("made {}", made.len());
}
fn source_left() {
    let v = vec![Loud(1), Loud(2), Loud(3)];
    let firsts: Vec<u8> = v.into_iter().flat_map(|l| vec![l.0, l.0 + 10]).take(3).collect();
    println!("{firsts:?}");
    let nested = vec![vec![Loud(20), Loud(21)], vec![Loud(22)], vec![Loud(23)]];
    let flat: Vec<Loud> = nested.into_iter().flatten().take(2).collect();
    println!("flat {}", flat.len());
}
fn both_left() {
    let w = vec![Loud(30), Loud(31)];
    let mut it = w.into_iter().flat_map(|l| vec![Loud(l.0 + 10), l]);
    let one = it.next();
    println!("one {}", one.map_or(0, |l| l.0));
    drop(it);
    let nested = vec![vec![Loud(50), Loud(51)], vec![Loud(52)]];
    let one: Vec<Loud> = nested.into_iter().flatten().take(1).collect();
    println!("one of nested {}", one.len());
}
fn options() {
    let v = [1, 2, 3];
    let big: Vec<i32> = v.iter().flat_map(|x| if *x > 1 { Some(*x) } else { None }).collect();
    let opts: Vec<Loud> = vec![Some(Loud(60)), None, Some(Loud(61))].into_iter().flatten().take(1).collect();
    println!("{big:?} {}", opts.len());
}
fn main() {
    inner_left();
    source_left();
    both_left();
    options();
    println!("end");
}
