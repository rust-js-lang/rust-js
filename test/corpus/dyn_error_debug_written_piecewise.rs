//@ nondeterministic: its run reads the file system
// A `dyn Error`'s `Debug` written into a writer of the crate's own, a
// piece at a time, as std's `fmt` of each error writes it (#2250).
use std::error::Error;
use std::fmt::{self, Write};
use std::io;

struct Pieces(Vec<String>);

impl Write for Pieces {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

// rand_core 0.5.1's error: a `dyn Error` it holds, shown by `{:?}`.
struct Wrapped {
    inner: Box<dyn Error + Send + Sync>,
}

impl fmt::Debug for Wrapped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Wrapped {{ inner: {:?} }}", self.inner)
    }
}

#[derive(Debug)]
struct Mine {
    code: i32,
    why: &'static str,
}

impl fmt::Display for Mine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "mine {} {}", self.code, self.why)
    }
}

impl Error for Mine {}

// An error only std's `io::Error::other` makes a `dyn` of, by the
// conversion it's given, which its body makes.
#[derive(Debug)]
struct Wrapper(&'static str);

impl fmt::Display for Wrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "wrapper {}", self.0)
    }
}

impl Error for Wrapper {}

fn show(name: &str, inner: Box<dyn Error + Send + Sync>) {
    let wrapped = Wrapped { inner };
    let mut plain = Pieces(Vec::new());
    write!(plain, "{wrapped:?}").unwrap();
    println!("{name}: {:?}", plain.0);
    let mut pretty = Pieces(Vec::new());
    write!(pretty, "{:#?}", wrapped.inner).unwrap();
    println!("{name} pretty: {:?}", pretty.0);
    println!("{name} text: {wrapped:?} / {:#?}", wrapped.inner);
}

fn main() {
    show("message", "bad \"thing\"\n".into());
    show("string", String::from("owned").into());
    show("parse int", Box::new("x".parse::<u8>().unwrap_err()));
    show("parse float", Box::new("x".parse::<f64>().unwrap_err()));
    show("parse bool", Box::new("x".parse::<bool>().unwrap_err()));
    show("try from int", Box::new(u8::try_from(300i32).unwrap_err()));
    show(
        "mine",
        Box::new(Mine {
            code: -7,
            why: "it's \"mine\"",
        }),
    );
    show("io kind", Box::new(io::Error::from(io::ErrorKind::NotFound)));
    show(
        "io new",
        Box::new(io::Error::new(io::ErrorKind::InvalidData, "oh \"no\"")),
    );
    show("io other", Box::new(io::Error::other(Wrapper("\"x\""))));
    show("io os", Box::new(std::fs::read("/no/such/file/at/all").unwrap_err()));
}
