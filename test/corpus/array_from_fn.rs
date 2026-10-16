// `array::from_fn(f)`: an array of `f`'s result for each index, of a length
// given or a caller's `N`.
fn ids<const N: usize>() -> [usize; N] {
    std::array::from_fn(|i| i + 1)
}

fn double(i: usize) -> usize {
    i * 2
}

fn main() {
    let squares: [u32; 4] = std::array::from_fn(|i| (i * i) as u32);
    let names: [String; 2] = std::array::from_fn(|i| format!("n{i}"));
    let doubled: [usize; 3] = std::array::from_fn(double);
    println!("{squares:?} {names:?} {doubled:?} {:?}", ids::<3>());
}
