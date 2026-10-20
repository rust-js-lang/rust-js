// A function made a `dyn Fn` by generic code alone, given a closure holding
// a value with a destructor: the `dyn Fn` drops it where Rust does (ADR
// 0521).
struct Guard(u32);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop guard {}", self.0);
    }
}

impl Guard {
    fn id(&self) -> u32 {
        self.0
    }
}

struct Registry {
    callbacks: Vec<Box<dyn Fn() -> u32>>,
}

impl Registry {
    fn add<F: Fn() -> u32 + 'static>(&mut self, f: F) {
        self.callbacks.push(Box::new(f));
    }
}

fn main() {
    let mut registry = Registry { callbacks: Vec::new() };
    for i in 1..=2 {
        let guard = Guard(i);
        registry.add(move || guard.id() * 10);
    }
    registry.add(|| 7);
    let sum: u32 = registry.callbacks.iter().map(|f| f()).sum();
    println!("sum {sum}");
    registry.callbacks.truncate(1);
    println!("truncated");
}
