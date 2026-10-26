// An `enumerate()` of a slice's items gives its index below the slice's
// length (#363): `levels[i]` of `for (i, &t) in terraces.iter().enumerate()`.
// A slice variable the body points elsewhere, or a function's items that
// outnumber the slice, keeps its check. What Rust does with it is what it
// does natively.

fn weighted(values: &[u32]) -> u32 {
    let weights = vec![2; values.len()];
    let mut sum = 0;
    for (i, &value) in values.iter().enumerate() {
        sum += weights[i] * value + values[i];
    }
    sum
}

fn ranked(items: &mut [u32]) {
    for (rank, item) in items.iter_mut().enumerate().rev() {
        *item = *item * 10 + rank as u32;
    }
}

fn rebound(a: &[u32], b: &[u32]) -> u32 {
    let mut xs = a;
    let mut sum = 0;
    for (i, _) in xs.iter().enumerate() {
        sum += xs.get(i).copied().unwrap_or(100);
        xs = b;
    }
    sum
}

fn doubled(xs: &[u32]) -> impl Iterator<Item = u32> + '_ {
    xs.iter().chain(xs.iter()).copied()
}

fn twice(xs: &[u32]) -> u32 {
    let mut sum = 0;
    for (i, x) in doubled(xs).enumerate() {
        sum += x * xs.get(i).copied().unwrap_or(1);
    }
    sum
}

fn main() {
    let mut items = vec![1, 2, 3];
    ranked(&mut items);
    println!("{} {items:?}", weighted(&[1, 2, 3]));
    println!("{} {}", rebound(&[1, 2, 3], &[7]), twice(&[2, 3]));
}
