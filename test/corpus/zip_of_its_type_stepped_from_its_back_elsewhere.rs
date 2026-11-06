// A map or a filter of a compiled `Zip`, std's struct, folded from its back
// where the crate folds a zip of that type from its back elsewhere: the
// zip is given its compiled back, `$knowing`, as it is where nothing else
// steps its type from its back. It was `$mapEnds` of the struct as it was
// made, whose back the type said it had, and the fold threw where Rust
// runs (#856).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn zipped() -> u32 {
    ds().into_iter()
        .zip(10..13u32)
        .rfold(0, |n, (d, i)| n * 100 + d.0 * 10 + i - 10)
}

fn mapped() -> u32 {
    ds().into_iter()
        .zip(10..13u32)
        .map(|(d, i)| d.0 + i)
        .rfold(0, |n, x| n * 100 + x)
}

fn filtered() -> u32 {
    ds().into_iter()
        .zip(10..13u32)
        .filter(|(d, _)| d.0 != 2)
        .rfold(0, |n, (d, i)| n * 100 + d.0 + i)
}

fn main() {
    println!("{}", zipped());
    println!("{}", mapped());
    println!("{}", filtered());
}
