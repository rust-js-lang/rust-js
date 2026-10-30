//@ library-differs
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct Inner;
impl fmt::Display for Inner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "inner")
    }
}
impl Error for Inner {}

#[derive(Debug)]
struct Outer(Inner);
impl fmt::Display for Outer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "outer")
    }
}
impl Error for Outer {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(Debug)]
struct Old;
impl fmt::Display for Old {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "old")
    }
}
#[allow(deprecated)]
impl Error for Old {
    fn description(&self) -> &str {
        "an old error"
    }
    fn cause(&self) -> Option<&dyn Error> {
        Some(&Inner)
    }
}

#[allow(deprecated)]
fn report<E: Error>(e: &E) -> String {
    let cause = match e.cause() {
        Some(c) => c.to_string(),
        None => String::from("none"),
    };
    format!("{e}: {} / {cause}", e.description())
}

#[allow(deprecated)]
fn shown(e: &dyn Error) -> String {
    let cause = e.cause().map_or(String::from("none"), |c| c.to_string());
    format!("{e}: {} / {cause}", e.description())
}

fn main() {
    println!("{}", report(&Inner));
    println!("{}", report(&Outer(Inner)));
    println!("{}", report(&Old));
    let errors: [Box<dyn Error>; 3] = [Box::new(Inner), Box::new(Outer(Inner)), Box::new(Old)];
    for e in &errors {
        println!("{}", shown(e.as_ref()));
    }
}
