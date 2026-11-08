// A closure that takes part of a value with a destructor, `move ||
// held.0.id()`, edition 2021's capture of a place: its drop drops that
// part, where it's dropped, given to generic code that drops it, kept in a
// `Box<dyn Fn>` or dropped itself, and the value's own drop drops the rest,
// as a part moved out, on the way the closure's made only. Given to
// generic code, it was taken for one that drops nothing, so its part was
// never dropped; made and dropped where it's made, it was refused (#1012).
struct Guard(u32);

impl Guard {
    fn id(&self) -> u32 {
        self.0
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop guard {}", self.0);
    }
}

struct Pair(Guard, Guard);

fn keep<F: Fn() -> u32 + 'static>(f: F) -> Box<dyn Fn() -> u32> {
    Box::new(f)
}

fn run<F: Fn() -> u32>(f: F) -> u32 {
    f()
}

fn once<F: FnOnce() -> Guard>(f: F) -> Guard {
    f()
}

fn branch(take: bool) {
    let held = Pair(Guard(10), Guard(11));
    if take {
        let c = move || held.0.id();
        println!("took {}", run(c));
    }
    println!("branch end");
}

// Its part dropped as the panic unwinds through what it's given to, then
// the rest of the value.
fn boom() -> u32 {
    let held = Pair(Guard(20), Guard(21));
    run(move || {
        if held.1.id() > 1 {
            panic!("boom");
        }
        held.1.id()
    })
}

fn main() {
    println!("{}", std::panic::catch_unwind(boom).is_err());
    let held = Pair(Guard(1), Guard(2));
    let kept = keep(move || held.0.id());
    println!("{}", kept());
    drop(kept);
    println!("after");
    let other = Pair(Guard(3), Guard(4));
    println!("{}", run(move || other.1.id() + other.0.id()));
    println!("mid");
    let third = Pair(Guard(5), Guard(6));
    let g = once(move || third.1);
    println!("got {}", g.id());
    let direct = Pair(Guard(7), Guard(8));
    let c = move || direct.0.id();
    println!("{}", c());
    drop(c);
    println!("direct dropped");
    branch(true);
    branch(false);
    println!("end");
}
