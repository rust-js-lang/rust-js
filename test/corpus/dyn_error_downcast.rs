//@ library-differs
// A `dyn Error`'s downcasts, `is`, `downcast_ref`, `downcast_mut` and a
// `Box`'s `downcast`, ask its dictionary's `type_id`, `Error`'s hidden
// method, as a `dyn Any`'s ask `Any`'s (ADR 0331): of the crate's own error,
// std's `io::Error`, a parse error, a message's, and a generic impl's value,
// whose dictionary has none, so no downcast to a type of no type parameter
// finds it. rand_core 0.5's `raw_os_error` downcasts its inner error to an
// `io::Error` so (#1400).
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
struct Mine(u32);

impl fmt::Display for Mine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "mine {}", self.0)
    }
}

impl Error for Mine {}

#[derive(Debug)]
struct Other;

impl fmt::Display for Other {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "other")
    }
}

impl Error for Other {}

/// A generic error, boxed but never downcast to.
#[derive(Debug)]
struct Wrapped<E>(E);

impl<E: fmt::Debug> fmt::Display for Wrapped<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "wrapped {:?}", self.0)
    }
}

impl<E: fmt::Debug> Error for Wrapped<E> {}

fn code(e: &(dyn Error + Send + Sync + 'static)) -> Option<i32> {
    e.downcast_ref::<io::Error>().and_then(io::Error::raw_os_error)
}

fn describe(e: &(dyn Error + 'static)) -> String {
    match (
        e.downcast_ref::<Mine>(),
        e.is::<Other>(),
        e.downcast_ref::<std::num::ParseIntError>(),
    ) {
        (Some(mine), _, _) => format!("mine of {}", mine.0),
        (None, true, _) => "an other".to_string(),
        (None, false, Some(parsed)) => format!("parsed: {parsed}"),
        _ => format!("unknown: {e}"),
    }
}

fn main() {
    let os: Box<dyn Error + Send + Sync> = Box::new(io::Error::from_raw_os_error(2));
    let message: Box<dyn Error + Send + Sync> = "text".into();
    println!("{:?} {:?}", code(&*os), code(&*message));

    let errors: Vec<Box<dyn Error>> = vec![
        Box::new(Mine(7)),
        Box::new(Other),
        Box::new("x".parse::<u8>().unwrap_err()),
        Box::new(Wrapped(3u8)),
        "plain".into(),
    ];
    for e in &errors {
        println!("{}", describe(&**e));
    }

    let mut boxed: Box<dyn Error> = Box::new(Mine(1));
    if let Some(mine) = boxed.downcast_mut::<Mine>() {
        mine.0 += 10;
    }
    match boxed.downcast::<Other>() {
        Ok(_) => println!("an other"),
        Err(e) => match e.downcast::<Mine>() {
            Ok(mine) => println!("got mine {}", mine.0),
            Err(e) => println!("still {e}"),
        },
    }
}
