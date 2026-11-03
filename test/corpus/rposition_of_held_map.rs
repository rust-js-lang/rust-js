// `rposition` of a `map` held in a variable, which steps it from its back
// and leaves it where it stopped, which the guide's limitations listed as
// refused after it compiled (nightly of 2026-11-03, #491).
fn main() {
    let v = [1, 2, 3];
    let mut it = v.iter().map(|x| x + 1);
    println!("{:?}", it.rposition(|x| x == 3));
    println!("{:?}", it.next());
    let mut it = v.iter().map(|x| x * 2);
    println!("{:?}", it.rposition(|x| x == 2));
    println!("{:?}", it.next());
    let mut it = v.iter().map(|x| x * 2);
    println!("{:?}", it.rposition(|x| x == 9));
    println!("{:?}", it.next());
}
