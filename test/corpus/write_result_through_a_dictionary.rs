//@ library-differs
// A write's result given back by a trait's method whose signature gives a
// type parameter, called through a dictionary: the value, `Err` or `Ok`,
// as Rust gives it, whether the impl holds it as a value or writes it, and
// `?` of it goes on with its `Err` (#1634). Where the bound names a type
// parameter or an associated type, `W: Is<T>`, `M::Out`, generic code
// holds it as a value, which is refused until the entry gives it as one
// (#2559, #2565: `valued_made_by_an_entry_of_a_trait_parameter`,
// `valued_made_by_an_entry_as_self_out`).
use std::fmt::{self, Write};

struct Failing;

impl fmt::Debug for Failing {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

struct Writes(bool);

impl Is<fmt::Result> for Writes {
    fn is(self) -> fmt::Result {
        let mut text = String::new();
        match self.0 {
            true => write!(text, "{:?}", Failing),
            false => write!(text, "fine"),
        }
    }
}

fn kept(written: impl Is<fmt::Result>) -> bool {
    let again = written.is();
    again.is_err()
}

fn dropped(written: impl Is<fmt::Result>) {
    let _again = written.is();
    println!("dropped");
}

fn checked(written: impl Is<fmt::Result>) -> fmt::Result {
    written.is()?;
    println!("checked on");
    Ok(())
}

fn main() {
    let mut text = String::new();
    let written = write!(text, "{:?}", Failing);
    println!("kept {}", kept(written));
    println!("kept ok {}", kept(Ok(())));
    println!("writes {} {}", kept(Writes(true)), kept(Writes(false)));
    dropped(written);
    dropped(Writes(true));
    println!("checked {:?} {:?}", checked(written), checked(Writes(false)));
    println!("static {:?}", Is::<fmt::Result>::is(written));
    println!("static writes {:?}", <Writes as Is<fmt::Result>>::is(Writes(true)));
    println!("end");
}
