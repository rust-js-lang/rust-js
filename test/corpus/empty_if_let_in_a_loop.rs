// An `if let` in a loop that binds and does nothing: tidy's `in place`
// stage reads the binding where MIR's temporary was, which leaves `if (_x
// != null) {}`, an `if` of nothing, which the stage takes away, as it
// ensures, keeping its test where that does anything (generated program
// 2900000355, ADR 0659).
fn id<T>(x: T) -> T {
    x
}

fn pick(n: u32) -> i8 {
    println!("pick {n}");
    if n % 2 == 0 { 1 } else { 2 }
}

fn main() {
    let mut turns = 0;
    for k in 0..3 {
        if let Some(_x) = Some(if id(k == 1) { 1 } else { 2 }) {}
        if let Some(_y) = Some(pick(k)) {}
        turns += 1;
    }
    println!("{turns}");
}
