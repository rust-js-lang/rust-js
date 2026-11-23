// A chain held in a variable that an `unzip()` into `Vec`s steps is made in
// place, as one given to it where it's made is: the helper fills its sides,
// stepping it as a JS iterator, not std's compiled body, which would hold it
// as std's structs (#2395).
fn main() {
    let pairs: Vec<(u32, u32)> = (0..4).map(|n| (n, n * 2)).collect();
    let it = pairs.into_iter().map(|(a, b)| (b, a + 1));
    let (a, b): (Vec<u32>, Vec<u32>) = it.unzip();
    println!("{a:?} {b:?}");
}
