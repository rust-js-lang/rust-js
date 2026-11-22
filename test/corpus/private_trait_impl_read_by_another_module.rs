// A private trait's impl, written in one module, given to generic code by
// another: its dictionary, the first module's `const`, is exported to the
// other, as a public trait's is, where it was left unexported and the
// other's import of it failed to load (#2323). So is a library's impl the
// crate compiles itself (ADR 0914), rand's `ChaCha20Core`'s in two of its
// modules.

trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> &'static str;
}

fn describe<S: Shape>(shape: &S) -> String {
    format!("{} of {}", shape.name(), shape.area())
}

mod shapes {
    pub struct Square(pub u32);

    impl super::Shape for Square {
        fn area(&self) -> u32 {
            self.0 * self.0
        }

        fn name(&self) -> &'static str {
            "a square"
        }
    }
}

mod report {
    use super::shapes::Square;

    pub fn lines() -> Vec<String> {
        [1, 3].into_iter().map(|side| super::describe(&Square(side))).collect()
    }
}

fn main() {
    for line in report::lines() {
        println!("{line}");
    }
}
