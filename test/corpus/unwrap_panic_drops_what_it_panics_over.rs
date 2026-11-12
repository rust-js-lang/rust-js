//@ run-fail: called `Result::unwrap()` on an `Err` value: Loud(11)
// What a `Result`'s `unwrap`, `expect`, `unwrap_err` or `expect_err` panics
// over is dropped as the panic unwinds, after its message is made: std's
// `unwrap_failed(msg, &e)` panics while `e` is a local of `unwrap`'s frame
// (#1505). Caught, in generic code given the drop, of a value whose drop
// glue drops two, of a box, and uncaught, as `main` unwinds.
use std::panic;

#[derive(Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug)]
struct Pair(Loud, Loud);

fn get(n: u32) -> Result<u32, Loud> {
    if n > 5 { Err(Loud(n)) } else { Ok(n) }
}

fn ok(n: u32) -> Result<Loud, u32> {
    Ok(Loud(n))
}

fn unwrapped<T, E: std::fmt::Debug>(r: Result<T, E>) -> T {
    r.unwrap()
}

fn main() {
    let r = panic::catch_unwind(|| get(7).unwrap());
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| get(8).expect("eight"));
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| ok(9).unwrap_err());
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| ok(10).expect_err("ten"));
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| unwrapped(get(12)));
    println!("caught generic {}", r.is_err());
    let r = panic::catch_unwind(|| {
        let p: Result<u32, Pair> = Err(Pair(Loud(13), Loud(14)));
        p.unwrap()
    });
    println!("caught pair {}", r.is_err());
    let r: Result<u32, Box<Loud>> = Err(Box::new(Loud(15)));
    let r = panic::catch_unwind(move || r.unwrap());
    println!("caught boxed {}", r.is_err());
    println!("{}", get(3).unwrap());
    println!("{}", unwrapped(get(4)));
    get(11).unwrap();
}
