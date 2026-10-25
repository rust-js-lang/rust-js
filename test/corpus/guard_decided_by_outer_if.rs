// A match arm's guard an `if` around it already decided, `if strict { ..
// (a, b) if strict => .. }`: the arm's value in its place, and what follows
// it, never reached, gone, as the stage that decides it ensures (#270).
fn read(n: i32) -> i32 {
    println!("read {n}");
    n
}

fn pick(strict: bool, x: i32, y: i32, fallback: i8) -> i8 {
    if strict {
        match (read(x), read(y)) {
            (5, _) => 5,
            (_, 2) => 2,
            (a, b) if strict => (a + b) as i8,
            _ => 127,
        }
    } else {
        fallback
    }
}

fn main() {
    println!("{}", pick(true, 5, 0, 9));
    println!("{}", pick(true, 0, 2, 9));
    println!("{}", pick(true, 3, 4, 9));
    println!("{}", pick(false, 0, 0, 9));
}
