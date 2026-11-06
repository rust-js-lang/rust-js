// A function only calls reach is told its parameters' ranges by each of
// them, joined (ADR 0577), however many there are and in whatever order
// they're walked (#866): `small`'s `n + 1` can't wrap, as each call gives
// it at most 101; `any`'s can, as one gives it `u32::MAX`, and wraps to 0.
fn small(n: u32) -> u32 {
    if n > 100 {
        panic!("too big: {n}");
    }
    n + 1
}

fn any(n: u32) -> u32 {
    n + 1
}

fn first(n: u32) -> u32 {
    small(n) + 1
}

fn second(n: u32) -> u32 {
    small(n) + 2
}

fn third(n: u32) -> u32 {
    small(n) + 3
}

fn fourth(n: u32) -> u32 {
    small(n) + 4
}

fn fifth(n: u32) -> u32 {
    small(n) + 5
}

fn main() {
    println!("{}", first(0));
    println!("{}", second(99));
    println!("{}", third(100));
    println!("{}", fourth(50));
    let caught = std::panic::catch_unwind(|| fifth(101));
    println!("caught {}", caught.is_err());
    for n in [0, 100, 101, u32::MAX - 1] {
        println!("any {n} {}", any(n));
    }
    println!("any max {}", any(u32::MAX));
}
