// A function only calls reach, told otherwise again and again (ADR 0577,
// #873): `walk`'s own call tells it a wider `n` each time it's walked,
// which nothing bounds, so it never settles; past four times what still
// changes is forgotten, `n`, and what it's told the same each time is kept,
// `k`, at most 7, so its `k + 1` can't overflow. Past 64 times all of it
// would be.
fn walk(n: u32, k: u8) -> u32 {
    if n % 7 == 3 {
        return (k + 1) as u32;
    }
    walk(n + 1, k) + 1
}

fn main() {
    println!("{}", walk(0, 3));
    println!("{}", walk(25, 7));
}
