//@ run-fail: boom at 2
// A call written where its value is used drops what's live as its panic
// unwinds, as any other does (ADR 0098).
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn boom(n: u32) -> u32 {
    if n > 1 {
        panic!("boom at {}", n);
    }
    n
}

fn main() {
    let _a = Noisy("a");
    let x = boom(1) + boom(2);
    println!("{x}");
}
