// An `io::Error` is a value (ADR 0462): made of a kind and an error or a
// message, or of its kind alone; its `kind()`, its `{}` and `{:?}`, plain and
// pretty, as std shows them, and it as a `dyn Error`.
use std::error::Error;
use std::io;

fn parse(s: &str) -> io::Result<u32> {
    s.parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn check(n: u32) -> Result<u32, io::Error> {
    if n > 10 {
        return Err(io::Error::other("too big"));
    }
    if n == 0 {
        return Err(io::ErrorKind::NotFound.into());
    }
    Ok(n)
}

fn main() {
    for s in ["5", "x", "50", "0"] {
        match parse(s).and_then(check) {
            Ok(n) => println!("ok {n}"),
            Err(e) => println!("{} | {:?} | {:?} | {} | {}", e, e.kind(), e, e.kind(), e.source().is_some()),
        }
    }
    let r: io::Result<u32> = Ok(3);
    println!("{}", r.expect("fine"));
    let e = io::Error::new(io::ErrorKind::InvalidInput, String::from("bad input"));
    println!("{e:#?}");
    let k: io::Error = io::Error::from(io::ErrorKind::TimedOut);
    println!("{k:#?} {k}");
    let boxed: Box<dyn Error> = Box::new(io::Error::other("inner"));
    println!("{boxed} {boxed:?}");
    let wrapped: Result<(), Box<dyn Error>> = Err(io::Error::new(io::ErrorKind::Other, "q").into());
    println!("{:?}", wrapped);
}
