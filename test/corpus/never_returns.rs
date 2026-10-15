//@ run-fail: empty
// A function that never returns, `-> !`, is called as any other: what
// follows its call isn't reached (ADR 0386).

fn fail(message: &str) -> ! {
    println!("failing: {message}");
    panic!("{message}");
}

fn forever() -> ! {
    loop {}
}

fn half(n: i32) -> i32 {
    if n % 2 != 0 {
        fail("odd");
    }
    n / 2
}

fn first(items: &[i32]) -> i32 {
    let Some(&item) = items.first() else { fail("empty") };
    item
}

pub fn main() {
    println!("{}", half(8));
    println!("{}", first(&[3, 4]));
    if half(2) == 7 {
        forever();
    }
    first(&[]);
}
