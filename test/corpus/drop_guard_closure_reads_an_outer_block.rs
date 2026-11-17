// A guard of a closure made in an inner block, `if n > 1 { .. }`, whose
// closure reads what the function declared before the block, `start`: the
// closure's body is where it's called, at the block's end, which sees the
// same `start` (#501, `js::scopes::reach`).
struct Finally<F: FnMut()>(F);

impl<F: FnMut()> Drop for Finally<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

fn counted(n: u32) -> u32 {
    let start = n * 2;
    let mut total = n;
    if n > 1 {
        let _guard = Finally(|| println!("counted from {start}"));
        total += start;
        println!("inside {total}");
    }
    total
}

fn main() {
    println!("{}", counted(3));
    println!("{}", counted(1));
}
