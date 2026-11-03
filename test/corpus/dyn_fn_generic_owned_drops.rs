// A generic function's closure made a `dyn Fn` by generic code alone,
// holding a value of its type parameter in a `Vec`, which std's own type
// owns by its `PhantomData`: given one with a destructor, the `dyn Fn`
// drops it where Rust does (ADR 0521, amended).
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn boxed<F: Fn() -> usize + 'static>(f: F) -> Box<dyn Fn() -> usize> {
    Box::new(f)
}

fn holding<K: 'static>(value: K) -> Box<dyn Fn() -> usize> {
    let shared = vec![value];
    boxed(move || shared.len())
}

fn main() {
    {
        let count = holding(Noisy("held"));
        println!("holds {}", count());
    }
    println!("dropped");
    let numbers = holding(5_u32);
    println!("holds {}", numbers());
}
