// `push`, `push_str` and `write!` give a `String` a new JS string, which
// its place gets (ADR 0034): a variable's, or a field through a `&mut`.
use std::fmt::Write;

struct Log {
    text: String,
}

impl Log {
    fn add(&mut self, line: &str) {
        self.text.push_str(line);
        self.text.push('\n');
    }
}

fn main() {
    let mut s = String::new();
    s.push_str("ab");
    s.push('c');
    write!(s, "-{}", 7).unwrap();
    let mut log = Log { text: String::new() };
    log.add("one");
    log.add(&s);
    print!("{}", log.text);
}
