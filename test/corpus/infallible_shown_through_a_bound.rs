//@ library-differs
// An `Infallible` error shown through a bound, `type Error: Debug +
// Display`, as rand_core 0.9's blanket `impl<R: RngCore> TryRngCore for R`
// gives it: no value of it exists, so what shows it never runs, its
// `Display` as its `Debug` (ADR 0161, #1100).
use std::convert::Infallible;
use std::fmt::{Debug, Display};

trait Source {
    fn next(&mut self) -> u32;
}

trait TrySource {
    type Error: Debug + Display;
    fn try_next(&mut self) -> Result<u32, Self::Error>;
}

impl<S: Source> TrySource for S {
    type Error = Infallible;
    fn try_next(&mut self) -> Result<u32, Infallible> {
        Ok(self.next())
    }
}

struct Counter(u32);

impl Source for Counter {
    fn next(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
}

fn described<T: TrySource>(source: &mut T) -> String {
    match source.try_next() {
        Ok(n) => format!("got {n}"),
        Err(e) => format!("failed: {e} ({e:?})"),
    }
}

fn shown<E: Display>(r: Result<u32, E>) -> String {
    match r {
        Ok(n) => n.to_string(),
        Err(e) => e.to_string(),
    }
}

fn main() {
    let mut c = Counter(0);
    println!("{}", described(&mut c));
    println!("{}", described(&mut c));
    println!("{}", shown(Ok::<u32, Infallible>(7)));
}
