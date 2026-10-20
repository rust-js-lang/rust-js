// A function made a `dyn Fn` drops what it holds where Rust drops the
// `dyn Fn`: a `Box`'s scope, a `Vec`'s `pop` and `remove`, a map's
// replaced value, the program's end (ADR 0521).
use std::collections::HashMap;

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

type Handler = Box<dyn Fn(u32) -> u32>;

fn adder(name: &'static str, by: u32) -> Handler {
    let noisy = Noisy(name);
    Box::new(move |x| {
        println!("{} adds {by}", noisy.name());
        x + by
    })
}

fn main() {
    {
        let f = adder("scoped", 1);
        println!("{}", f(1));
    }
    println!("after scope");
    let mut handlers: Vec<Handler> = vec![adder("first", 2), adder("second", 3)];
    let plain: Handler = Box::new(|x| x * 2);
    handlers.push(plain);
    let total = handlers.iter().fold(1, |acc, f| f(acc));
    println!("total {total}");
    let popped = handlers.pop();
    println!("popped {}", popped.is_some());
    drop(popped);
    let mut named: HashMap<&str, Handler> = HashMap::new();
    named.insert("a", adder("a1", 1));
    named.insert("a", adder("a2", 2));
    println!("replaced");
    drop(handlers.remove(0));
    println!("end");
}
