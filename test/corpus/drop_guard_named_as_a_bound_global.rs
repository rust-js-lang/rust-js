// A value kept only to be dropped, named `_escape`, beside a binding of
// the global function `escape` the body calls: the guard, read once it's
// inlined, is named apart from the global, `escape$1`, so the call reads
// the global still, never the guard (`parameter_names::unmarked`,
// ADR 1150).
struct Guard(&'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "escape")]
fn escape(text: &str) -> String {
    text.replace(' ', "%20")
}

fn main() {
    let _escape = Guard("guard");
    println!("{}", escape("a b"));
}
