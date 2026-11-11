// An `io::Error` made of an error owns it, and drops it where Rust drops
// the `io::Error`: of a local, a `Result` given back through `?`, a `Vec` of
// them, one made by `other`, one held by another, one boxed as a `dyn
// Error`, and rand_core 0.5's `From<Error> for io::Error` of its error,
// which holds a `Box<dyn Error + Send + Sync>`. `io::Error::new` of a value
// with a destructor was refused (#1403).
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
struct Loud(u32);

impl fmt::Display for Loud {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "loud {}", self.0)
    }
}

impl Error for Loud {}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

/// rand_core 0.5's error, of any error.
#[derive(Debug)]
struct RandError {
    inner: Box<dyn Error + Send + Sync>,
}

impl fmt::Display for RandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rand: {}", self.inner)
    }
}

impl Error for RandError {}

impl From<RandError> for io::Error {
    fn from(error: RandError) -> Self {
        io::Error::new(io::ErrorKind::Unsupported, error)
    }
}

fn fails(n: u32) -> io::Result<u32> {
    if n > 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, Loud(n)));
    }
    Ok(n * 10)
}

fn through(n: u32) -> io::Result<u32> {
    let got = fails(n)?;
    println!("got {got}");
    Ok(got + 1)
}

fn wrote(n: u32) -> io::Result<()> {
    if n > 2 {
        return Err(io::Error::other(Loud(n + 100)));
    }
    Ok(())
}

/// `?` of a write's result, whose `ControlFlow` holds nothing to drop.
fn wrote_twice(n: u32) -> io::Result<()> {
    wrote(1)?;
    wrote(n)?;
    println!("wrote twice");
    Ok(())
}

fn rand_fill() -> io::Result<()> {
    Err(RandError {
        inner: Box::new(Loud(40)),
    })?
}

fn main() {
    {
        let e = io::Error::new(io::ErrorKind::TimedOut, Loud(1));
        println!("{e} {:?}", e.kind());
    }
    println!("{:?}", through(1));
    match through(3) {
        Ok(n) => println!("ok {n}"),
        Err(e) => println!("err {e}"),
    }
    println!("after through");

    let errors: Vec<io::Error> = (10..13).map(|n| io::Error::other(Loud(n))).collect();
    println!("{}", errors.len());
    drop(errors);
    println!("after vec");

    let nested = io::Error::new(io::ErrorKind::Interrupted, io::Error::other(Loud(20)));
    println!("{nested}");
    let boxed: Box<dyn Error + Send + Sync> = Box::new(nested);
    println!("boxed {boxed}");
    drop(boxed);
    println!("after boxed");

    println!("{}", wrote_twice(1).is_ok());
    if let Err(e) = wrote_twice(5) {
        println!("{e}");
    }

    let plain = io::Error::other("no destructor");
    println!("{plain}");
    if let Err(e) = rand_fill() {
        println!("{e}");
    }
    println!("end");
}
