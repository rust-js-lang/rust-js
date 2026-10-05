//@ run-fail: first or last weekday is out of range: 9
// `panic!("{}", x)`, which std writes as `panic_display(&x)`: its message
// is `x`'s `Display`, as chrono's are.
fn pick(v: Option<u32>, msg: &str) -> u32 {
    match v {
        Some(n) => n,
        None => panic!("{}", msg),
    }
}

fn main() {
    println!("{}", pick(Some(4), "none"));
    let day = 9;
    let message = format!("first or last weekday is out of range: {day}");
    println!("{}", pick(None, &message));
}
