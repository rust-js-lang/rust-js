// What a `Box`, an `Rc` or an `impl Display` a function gives shows,
// with a placeholder's options: its `fmt` handed them, padded as what it
// holds is (#171, #197, #1109).
use std::fmt::{self, Display};
use std::rc::Rc;

struct Tag(u32);

impl Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&format!("t{}", self.0))
    }
}

fn shown() -> impl Display {
    Tag(7)
}

fn both() -> impl Display + fmt::Debug {
    "both"
}

fn main() {
    println!("[{:>4}] [{:<5}]", Box::new("s"), Rc::new("r"));
    println!("[{:>6}] [{}]", shown(), shown());
    println!("[{:?}] [{:>8}]", both(), both());
    println!("[{:>5?}] [{:^7}]", Box::new(3), Box::new(Tag(2)));
}
