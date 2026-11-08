//@ compile-fail: code that may be seen runs before it's awaited
// A future of the crate's made into a field of a local, held while
// something that prints runs, then awaited: as a future held in a variable
// (#965, ADR 0029 amended), rust-js follows it there and refuses it.
async fn greet() -> u32 {
    println!("hello");
    1
}

struct Slot<F> {
    f: Option<F>,
}

pub async fn later() -> u32 {
    let mut slot = Slot { f: None };
    slot.f = Some(greet());
    println!("first");
    match slot.f {
        Some(f) => f.await,
        None => 0,
    }
}

fn main() {
    let _ = later;
}
