// A closure's own parameter of a temporary's name, `(_2) => _2[0]`, where
// main's match binds `_2` too: naming main's temporary leaves the
// closure's reads its own (#1105).
fn id<T>(x: T) -> T {
    x
}

fn main() {
    let level: u16 = match (id(0u8), id(2i64)) {
        (255, _) => 297,
        (_, 1) => 1,
        (low, high) if id(true) => low as u16 + high as u16,
        _ => 0,
    };
    let sums: Vec<u64> = vec![1u64, 2]
        .into_iter()
        .zip(vec![40u64, 50])
        .map(|(a, b): (u64, u64)| a.wrapping_add(b))
        .collect();
    println!("{level} {sums:?}");
}
