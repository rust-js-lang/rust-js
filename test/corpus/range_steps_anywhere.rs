// A range's `step_by` and `skip` step it by its `nth`, of its bounds, as
// std's do, wherever the range comes from (#231): held in a variable,
// through generic code, of a start tidy can't read, reversed or skipped
// first; a `char`'s, by code point, still each.
fn every<I: Iterator<Item = u32>>(it: I, n: usize) -> Vec<u32> {
    it.step_by(n).collect()
}

fn main() {
    let held = 0..u32::MAX;
    let v: Vec<u32> = held.step_by(1 << 30).collect();
    println!("{v:?}");
    println!("{:?}", every(0..u32::MAX, 1 << 30));
    let start = std::hint::black_box(0u32);
    println!("{:?}", (start..u32::MAX).skip(4_000_000_000).next());
    let back = (0..u32::MAX).rev();
    let w: Vec<u32> = back.step_by(1 << 30).collect();
    println!("{w:?}");
    let skipped = (0..u32::MAX).skip(3);
    println!("{:?}", skipped.step_by(1 << 31).collect::<Vec<_>>());
    println!("{:?}", ('a'..'k').step_by(3).collect::<String>());
}
