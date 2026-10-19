// `hint::assert_unchecked(c)`, deranged's promise that a value is in its
// range: nothing in JS, its condition made as it's given.

fn loud(n: u32) -> u32 {
    println!("checked {n}");
    n
}

fn in_range(value: u32) -> u32 {
    // SAFETY: every caller gives a value up to 9.
    unsafe { std::hint::assert_unchecked(loud(value) <= 9) };
    value * 2
}

// As deranged writes it, of what does nothing as it's made.
fn bounded<const MIN: u32, const MAX: u32>(value: u32) -> u32 {
    // SAFETY: every caller gives a value in range.
    unsafe { std::hint::assert_unchecked(MIN <= value && value <= MAX) };
    value
}

fn main() {
    println!("{} {}", in_range(3), in_range(9));
    println!("{}", bounded::<1, 5>(4));
}
