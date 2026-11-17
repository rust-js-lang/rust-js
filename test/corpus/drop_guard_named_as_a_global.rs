// A value kept only to be dropped, named `_console`, `_window` or
// `_undefined`: its `_` stays where what's left is a global's name, which
// the body's and the drop's own prints read, `console.log`, so neither
// reads the guard (`parameter_names::unmarked`).
struct Guard(&'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let _console = Guard("console");
    let _window = Guard("window");
    let _undefined = Guard("undefined");
    let n = 7;
    println!("body {n} {:?}", None::<u8>);
}
