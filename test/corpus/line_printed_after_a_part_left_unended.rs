// A line printed after a part a `print!` left unended is that line's rest,
// of each stream: `ab` of `print!("a"); println!("b")`, natively and in a
// browser's console, which takes whole lines, so the line is printed
// through what the runtime holds, not by `console.log` past it (#1599).
// test/stream-order.test.ts holds both streams' order, and a browser's.
fn main() {
    print!("a");
    println!("b");
    eprint!("x");
    eprintln!("y");
    println!("c");
    print!("left ");
    print!("open");
}
