// `.rev()` of a `chain()` of a `map()` of a slice's items and another
// slice's copied: each side stepped from its back, `b`'s first, as std's
// `Chain` does; a map's of a `Vec`'s owned items too, and one folded from
// its back (#851, once refused).
fn main() {
    let v = [1, 2];
    let w = [3];
    let r: Vec<i32> = v.iter().map(|x| x * 10).chain(w.iter().copied()).rev().collect();
    println!("{r:?}");
    let owned = vec![String::from("a"), String::from("b")];
    let tail = ["c", "d"];
    let joined: Vec<String> = owned
        .into_iter()
        .map(|s| s + "!")
        .chain(tail.iter().map(|s| s.to_string()))
        .rev()
        .collect();
    println!("{joined:?}");
    let folded = v
        .iter()
        .map(|x| x + 1)
        .chain(w.iter().copied())
        .rfold(0, |acc, x| acc * 10 + x);
    println!("{folded}");
    let mut stepped = v.iter().map(|x| x * 2).chain(w.iter().copied());
    println!("{:?} {:?} {:?}", stepped.next_back(), stepped.next(), stepped.next_back());
}
