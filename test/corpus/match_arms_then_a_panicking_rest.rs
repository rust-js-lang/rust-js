// A `match` of or-patterns and a range, its rest a panic: the tests that
// leave to the panic are one guard, `if (day < 1 || day > 5) { throw .. }`,
// then what the range gives, as a person writes it (tidy's `guarded`).

fn weekend(day: u8) -> bool {
    match day {
        0 | 6 => true,
        1..=5 => false,
        _ => panic!("no day {day}"),
    }
}

fn main() {
    println!("{} {} {}", weekend(0), weekend(3), weekend(6));
    let r = std::panic::catch_unwind(|| weekend(9));
    println!("panicked {}", r.is_err());
}
