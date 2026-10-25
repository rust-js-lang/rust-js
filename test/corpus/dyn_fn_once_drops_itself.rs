// A closure made a `Box<dyn FnOnce()>` carries its drop (ADR 0521), and,
// called, its own body drops what it took as it ends, or as a panic
// unwinds it: there it drops what it holds, not the function it is, which
// has no name in its body. Dropped uncalled, its carried drop runs.
use std::panic::{self, AssertUnwindSafe};

struct Noisy(&'static str);

impl Noisy {
    fn name(&self) -> &'static str {
        self.0
    }
}

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn once(name: &'static str, fail: bool) -> Box<dyn FnOnce()> {
    let noisy = Noisy(name);
    Box::new(move || {
        if fail {
            panic!("{} failed", noisy.name());
        }
        println!("ran {}", noisy.name());
    })
}

fn run(f: Box<dyn FnOnce()>) {
    f();
}

fn main() {
    run(once("called", false));
    let unwound = panic::catch_unwind(AssertUnwindSafe(|| run(once("unwound", true))));
    println!("unwound: {}", unwound.is_err());
    drop(once("uncalled", false));
    println!("end");
}
