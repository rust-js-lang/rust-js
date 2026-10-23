// A `dyn Any` drops what it holds where the crate makes one of a value with
// a destructor (ADR 0691), here a type parameter's, given a `Noisy`: each
// drops it, a payload resumed of one too, and a caught panic's payload, of
// its message, drops nothing, as in Rust.
use std::any::Any;
use std::panic::{self, catch_unwind};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn describe(n: u32) -> String {
    catch_unwind(|| {
        if n > 2 {
            panic!("too big");
        }
        format!("small {n}")
    })
    .unwrap_or_default()
}

fn boxed<T: Any + Send>(value: T) -> Box<dyn Any + Send> {
    Box::new(value)
}

fn main() {
    let _guard = Noisy("guard");
    println!("{:?} {:?}", describe(1), describe(3));
    let any = boxed(Noisy("boxed"));
    println!("is noisy: {}", any.is::<Noisy>());
    drop(any);
    let number = boxed(5u8);
    println!("five: {:?}", number.downcast_ref::<u8>());
    let resumed = catch_unwind(|| -> u32 { panic::resume_unwind(boxed(Noisy("payload"))) });
    println!("resumed: {}", resumed.is_err());
    drop(resumed);
    println!("end");
}
