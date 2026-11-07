// `Option::inspect`, `Result::inspect` and `inspect_err` own what they give
// back: it's dropped as the closure panics. `unwrap_or_default` of an
// `Err` drops it as the default panics, as it does once the default is
// made (#903). A closure that can't panic reads as before.
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Debug)]
struct D(u32);

impl Default for D {
    fn default() -> D {
        println!("default");
        panic!("default");
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    run("Some", || {
        let o = Some(D(1)).inspect(|d| panic!("saw {}", d.0));
        println!("{o:?}");
    });
    run("Ok", || {
        let r: Result<D, u32> = Ok(D(2));
        let r = r.inspect(|d| panic!("saw {}", d.0));
        println!("{r:?}");
    });
    run("Err", || {
        let r: Result<u32, D> = Err(D(3));
        let r = r.inspect_err(|d| panic!("saw {}", d.0));
        println!("{r:?}");
    });
    run("unwrap_or_default", || {
        let r: Result<D, D> = Err(D(4));
        let d = r.unwrap_or_default();
        println!("{}", d.0);
    });
    run("quiet", || {
        let o = Some(D(5)).inspect(|d| println!("saw {}", d.0));
        println!("{o:?}");
    });
}
