//@ run-fail: index out of bounds: the len is 2 but the index is 5
// `v[i] = v[i] + ..` indexes one place twice, at two columns: each is
// its own checked item, never one `v[i] += ..`, as the right side is read
// first and its panic names its column, as native Rust's does; one place
// indexed once, `v[0].push(..)`, is one item read and written (#2334).
fn weighted_by_its_rather_long_name(first: u32, second: u32) -> u32 {
    first * 3 + second
}

fn main() {
    let mut totals = vec![1u32, 2];
    totals[0] = totals[0] + 1;
    println!("{totals:?}");
    let index = std::hint::black_box(5);
    totals[index] = totals[index] + weighted_by_its_rather_long_name(index as u32, totals.len() as u32);
    println!("{totals:?}");
}
