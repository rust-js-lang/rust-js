//@ run-fail: capacity overflow
// A collect into a `Vec` of `0..=usize::MAX`, whose length is past
// `usize::MAX`, so its hint has no upper bound, panics where std's
// `TrustedLen` collect does, `spec_from_iter_nested.rs`'s line, not where
// a `Vec` of too many bytes asks its room, raw_vec's (#2488).
fn main() {
    let few: Vec<usize> = (0..=3).collect();
    println!("{few:?}");
    let all: Vec<usize> = (0..=usize::MAX).collect();
    println!("{}", all.len());
}
