// A value kept only to be dropped, named `_in` or `_eval`, is read by a
// name JS lets it have: its `_` taken off only where what's left isn't a
// word JS keeps, `in`, `new`, `eval`, `arguments`.
struct Guard(&'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let _in = Guard("in");
    let _new = Guard("new");
    let _eval = Guard("eval");
    let _arguments = Guard("arguments");
    let _restore = Guard("restore");
    println!("body");
}
