// A `Fn` or `FnMut` closure made a `Box<dyn FnOnce()>` and called drops
// what it took as the call returns, as std's `call_once` consumes the box
// (#409); dropped uncalled, it drops then; an `FnOnce` closure drops in
// its own body, once; a call that panics drops as it unwinds.

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn run(task: Box<dyn FnOnce()>) {
    println!("calling");
    task();
    println!("called");
}

fn main() {
    let d = D(1);
    let shared = move || {
        let d = &d;
        println!("run {}", d.0);
    };
    run(Box::new(shared));

    let e = D(2);
    let unused: Box<dyn FnOnce()> = Box::new(move || {
        let e = &e;
        println!("never {}", e.0);
    });
    println!("made");
    drop(unused);

    let mut count = 0;
    let f = D(3);
    let counting = move || {
        count += 1;
        let f = &f;
        println!("count {} {}", count, f.0);
    };
    run(Box::new(counting));

    let g = D(4);
    run(Box::new(move || {
        let owned = g;
        println!("owned {}", owned.0);
    }));

    let h = D(5);
    let failing = move || {
        let h = &h;
        println!("failing {}", h.0);
        panic!("failed");
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(Box::new(failing))));
    println!("{}", result.is_err());

    // A `dyn Fn` made a `dyn FnOnce` is consumed by its call too.
    let i = D(6);
    let shared: Box<dyn Fn()> = Box::new(move || {
        let i = &i;
        println!("shared {}", i.0);
    });
    shared();
    run(shared);
    println!("end");
}
