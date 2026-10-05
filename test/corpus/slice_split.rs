// `split_at`, `split_at_checked` and `get` of a range, of a slice: each half or part a copy, as
// a shared slice is, and `None` where `&v[range]` would panic.

fn halves(bytes: &[u8]) -> (u32, u32) {
    let (high, low) = bytes.split_at(bytes.len() / 2);
    let fold = |part: &[u8]| part.iter().fold(0u32, |acc, b| acc * 256 + *b as u32);
    (fold(high), fold(low))
}

// Each item in turn, as chrono's queue of format items is read.
fn total(items: &[u32]) -> u32 {
    match items.split_first() {
        Some((first, rest)) => first + total(rest),
        None => 0,
    }
}

fn main() {
    let v = [1, 2, 3, 4, 5];
    let (left, right) = v.split_at(2);
    println!("{:?} {:?}", left, right);
    let (none, all) = v.split_at(0);
    let (whole, empty) = v[1..].split_at(4);
    println!("{:?} {:?} {:?} {:?}", none, all, whole, empty);
    println!("{:?}", halves(&[1, 0, 0, 2]));
    println!("{:?} {:?}", v.split_at_checked(2), v.split_at_checked(6));
    let words = vec![String::from("a"), String::from("b"), String::from("c")];
    let (first, rest) = words.split_at(1);
    println!("{} {}", first.join("+"), rest.join("+"));

    println!("{:?} {:?} {:?}", v.get(1..3), v.get(3..), v.get(..2));
    println!("{:?} {:?} {:?}", v.get(..), v.get(4..9), v.get(2..=4));
    let (start, end) = (3, 1);
    println!("{:?} {:?}", v.get(start..end), v.get(5..));
    println!("{:?} {:?} {:?}", v.split_first(), v.split_last(), v[..0].split_first());
    println!("{} {:?}", total(&v), words.split_last().map(|(last, rest)| (last.len(), rest.len())));
    let one = [9];
    println!("{:?} {:?}", one.split_first(), one.split_last());
    match words.get(1..) {
        Some(tail) => println!("{}", tail.len()),
        None => println!("none"),
    }
}
