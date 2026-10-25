// A thread-local made where it's first used (ADR 0037, amended) that's
// `set` before then is made of what it's given, its initializer never run
// and nothing of it dropped, as std's `LocalKey::set` (#252); and one used
// by a call is made after what that call is given, as Rust makes it inside
// the call: `replace(made(9))` makes 9, then the thread-local.
use std::cell::{Cell, RefCell};

fn made(n: u32) -> u32 {
    println!("made {n}");
    n
}

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

thread_local! {
    static COUNT: Cell<u32> = Cell::new(made(1));
    static OTHER: Cell<u32> = Cell::new(made(2));
    static HELD: Cell<Option<Noisy>> = Cell::new({
        made(3);
        Some(Noisy("first"))
    });
    static LIST: Cell<Vec<u32>> = Cell::new({
        made(7);
        Vec::new()
    });
    static NAME: RefCell<String> = RefCell::new({
        made(4);
        String::from("a")
    });
}

fn main() {
    COUNT.set(made(5));
    println!("{}", COUNT.get());
    COUNT.set(6);
    println!("{}", COUNT.get());

    let old = OTHER.replace(made(9));
    println!("{old} {}", OTHER.get());

    HELD.set(Some(Noisy("second")));
    HELD.set(None);
    println!("{}", HELD.take().is_none());

    // What's given made by more than one statement, `vec!`'s.
    LIST.set(vec![made(8), 2]);
    println!("{:?}", LIST.take());

    NAME.set(String::from("b"));
    println!("{}", NAME.with_borrow(|n| n.clone()));
    println!("end");
}
