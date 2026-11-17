// A variable and its shadow, `let a` twice in one function, each moved on
// some paths only: each has its own drop flag, spelled apart where both
// are in scope, `a$live` and `a$live$1` or as preparation spells them, so
// each drop runs exactly where Rust's does (ADR 1150).
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn consume(n: Noisy) {
    println!("took {}", n.0);
}

fn run(first: bool, second: bool) {
    let a = Noisy("outer");
    if first {
        consume(a);
    }
    let a = Noisy("inner");
    if second {
        consume(a);
    }
    println!("end {first} {second}");
}

fn main() {
    run(false, false);
    run(true, false);
    run(false, true);
    run(true, true);
}
