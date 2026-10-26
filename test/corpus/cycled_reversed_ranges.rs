// What's held to be cloned is std's struct, which its compiled `Clone`
// clones (ADR 0915): a `rev()` a `cycle()` holds, and one a chain cloned
// apart holds, as a cycled zigzag's (#308).
fn zigzag(n: usize, len: usize) -> Vec<usize> {
    (0..n - 1).chain((1..n).rev()).cycle().take(len).collect()
}

fn main() {
    println!("{:?}", zigzag(3, 9));
    println!("{:?}", zigzag(4, 8));
    let back: Vec<u8> = (1..4u8).rev().cycle().take(5).collect();
    println!("{back:?}");
    let skipped: Vec<i32> = [5, 6, 7].iter().copied().skip(1).cycle().take(5).collect();
    println!("{skipped:?}");
}
