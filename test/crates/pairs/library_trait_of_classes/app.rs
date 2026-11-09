// A consumer of a library's trait of classes, which calls its methods on
// the library's class, as the library's prototype has them, and implements
// it for a class of its own, whose prototype then has them too, the
// trait's default of its library's MIR (#1073).
use dep::{Render, Root};

#[cfg_attr(rust_js, rust_js::class)]
pub struct Leaf {
    n: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Leaf {
    pub fn new(n: u32) -> Self {
        Leaf { n }
    }
}

impl Render for Leaf {
    fn label(&self) -> String {
        format!("[{}]", self.n)
    }
}

fn twice(item: &Root) -> String {
    format!("{} {}", item.label(), item.render(1))
}

pub fn main() {
    let root = dep::root("a");
    println!("{}", root.label());
    println!("{}", root.render(3));
    println!("{}", dep::shown(&root));
    println!("{}", twice(&root));
    println!("{}", twice(&Root::new("b")));
    let leaf = Leaf::new(3);
    println!("{} {}", leaf.label(), leaf.render(2));
}
