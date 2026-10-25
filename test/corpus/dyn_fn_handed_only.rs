// Where every function holding a value with a destructor that the crate
// makes a `dyn Fn` is handed to JS, which keeps it, no `dyn Fn` Rust drops
// has anything to drop (ADRs 0521, 0871): `ignored` drops its `Box<dyn Fn>`
// as nothing, `$dispose` of none.

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

/// `task.call()`: JS calls the task and keeps it; nothing drops it.
#[cfg_attr(rust_js, rust_js::link_name = "call")]
fn run_kept(this: Box<dyn Fn()>) {
    this();
    std::mem::forget(this);
}

fn ignored(task: Box<dyn Fn()>, run: bool) {
    if run {
        task();
    }
}

fn main() {
    let noisy = Noisy("handed");
    run_kept(Box::new(move || println!("run {}", noisy.name())));
    ignored(Box::new(|| println!("run plain")), true);
    ignored(Box::new(|| println!("never")), false);
    println!("end");
}
