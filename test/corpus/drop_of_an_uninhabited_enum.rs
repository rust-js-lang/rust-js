// An enum of no variants with a destructor (rustc's
// drop/drop-uninhabited-enum.rs, #916): no value of it is ever made, so
// its drop never runs. What holds one, an `Option` that's `None`, an empty
// `Vec`, a `Result` that's `Ok` and a struct of them, drops as it would
// of any type, its own fields dropped, none of them a `Never`.
struct Never(Empty);

enum Empty {}

impl Drop for Empty {
    fn drop(&mut self) {
        unreachable!("no value of an empty enum");
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Holder {
    name: Loud,
    maybe: Option<Never>,
    many: Vec<Empty>,
}

/// Takes one by value, which it drops at its end, as rustc's test does.
fn discard(_value: Empty) {}

fn checked(n: u32) -> Result<u32, Empty> {
    Ok(n * 2)
}

fn main() {
    let none: Option<Empty> = None;
    println!("none {}", none.is_none());
    drop(none);
    let many: Vec<Empty> = Vec::new();
    println!("many {}", many.len());
    drop(many);
    let n = checked(4).map_err(discard).unwrap_or(0);
    println!("ok {n}");
    let holder = Holder { name: Loud("holder"), maybe: None, many: Vec::new() };
    println!("{} {} {}", holder.name.0, holder.maybe.is_none(), holder.many.len());
    drop(holder);
    println!("end");
}
