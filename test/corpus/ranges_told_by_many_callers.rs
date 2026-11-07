// A function only calls reach is told its parameters' ranges by each of
// them, joined (ADR 0577), however many there are (#866): `small`'s six
// callers, each giving it more than the one before, tell it before it's
// walked again, so its `n` isn't forgotten as one that keeps changing,
// and its `n + 1` can't wrap, as each call gives it at most 101. Nothing
// but its callers says so: a guard that panics would narrow `n` itself
// (#867). `any`'s can, as one gives it `u32::MAX`, and wraps to 0.
fn small(n: u32) -> u32 {
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

fn sixth(n: u32) -> u32 {
    small(n) + 6
}

fn main() {
    println!("{}", first(0));
    println!("{}", second(20));
    println!("{}", third(40));
    println!("{}", fourth(60));
    println!("{}", fifth(100));
    println!("{}", sixth(101));
    for n in [0, 100, 101, u32::MAX - 1] {
        println!("any {n} {}", any(n));
    }
    println!("any max {}", any(u32::MAX));
}
