// A reversed range skipped, `(0..u64::MAX).rev().skip(4_000_000_000)`, beside a
// function that steps a reversed range of the same type from its back,
// which makes that type's `rev()` one made knowing its back, `$rev` of the
// range: its `skip` asks the `rev()`'s `nth`, std's `Rev::nth`, the
// range's `nth_back`, which moves its end, as std's does, so it doesn't
// step each of the 4e9 numbers it skips, 15 s of each, where Rust's is
// instant (#950).
fn second_from_its_front(range: std::ops::Range<u64>) -> Option<u64> {
    range.rev().nth_back(1)
}

fn main() {
    println!("{:?}", (0..u64::MAX).rev().skip(4_000_000_000).next());
    println!("{:?}", (0..u32::MAX).rev().skip(4_000_000_000).next());
    println!("{:?}", second_from_its_front(3..9));
}
