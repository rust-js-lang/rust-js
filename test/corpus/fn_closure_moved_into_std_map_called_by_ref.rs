// A closure rustc infers `Fn`, holding a value with a destructor, bound by
// `let` and moved into std's `map`, which calls it by `&mut` for each item
// and drops it once the sum ends (#2031's leave-alone): no call by value
// reaches it, so its body is its own and std's adapter drops it.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let d = Loud("mapped");
    let shown = move |x: u32| {
        let held = &d;
        println!("in {} {}", held.0, x);
        x + 1
    };
    println!("{}", vec![1u32, 2].into_iter().map(shown).sum::<u32>());
    println!("end");
}
