//! Each placeholder's options as Rust applies them.
use dep::{Hour, Joined, Message, Row};

pub fn main() {
    let j = Joined(&[10, 255], ", ");
    println!("{j} {j:x} {j:#x} [{j:>4}] {j:?}");
    println!("{:?} {:#?}", Row(vec![1, 2]), Row(vec![3]));
    println!("[{:>5}]", Hour(7));
    println!("{:?}", Message(String::from("say \"hi\"\n")));
}
