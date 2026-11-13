// An `fmt::Arguments` given to a generic std function that keeps it,
// `Box::new`, `Vec::push`, `Rc::new`, is the function that makes its text,
// as any `Arguments` kept is, so where it's read it's formatted (ADR 0034,
// #1726). Only a literal's outlives its statement there.
use std::fmt::{self, Display};
use std::rc::Rc;

struct Boxed<'a>(Box<fmt::Arguments<'a>>);

impl Display for Boxed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&*self.0, f)
    }
}

fn main() {
    let boxed = Boxed(Box::new(format_args!("boxed")));
    println!("{boxed} {:>8}|", boxed);

    let mut kept = Vec::new();
    kept.push(format_args!("first"));
    kept.push(format_args!("second"));
    for args in &kept {
        println!("{args} {}", args.to_string().len());
    }
    let rc = Rc::new(format_args!("rc"));
    println!("{} {}", rc, Rc::strong_count(&rc));
}
