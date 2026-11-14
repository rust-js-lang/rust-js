//@ library-differs
// A `PhantomData<T>`'s `Debug` written into a writer a piece at a time,
// `write!(f, "PhantomData<{}>", type_name::<T>())`'s three writes (#1712):
// directly, `{:#?}` too, as a derived `Debug`'s field, and of a type
// parameter, its caller's name.
use std::fmt::{self, Write};
use std::marker::PhantomData;

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

#[derive(Debug)]
struct Tagged<U> {
    n: u8,
    of: PhantomData<U>,
}

fn tagged<U>(n: u8) -> Tagged<U> {
    Tagged { n, of: PhantomData }
}

fn written<U: fmt::Debug>(log: &mut Log, n: u8) {
    write!(log, "{:?}", tagged::<U>(n)).unwrap();
}

fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{:?}|{:#?}", tagged::<u32>(1), PhantomData::<String>).unwrap();
    written::<Vec<bool>>(&mut log, 2);
    println!("{:?}", log.0);
}
