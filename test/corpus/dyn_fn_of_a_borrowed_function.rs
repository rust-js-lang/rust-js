// A `dyn Fn` of a borrowed function, `Rc::new(&f)`'s, drops nothing of
// what it borrows: its drop is the reference's. And a `dyn Fn` boxed again
// drops what it holds once, as its own drop (ADR 0521, amended).
use std::rc::Rc;

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn greeter(name: &'static str) -> Box<dyn Fn(u32) -> u32> {
    let noisy = Noisy(name);
    Box::new(move |x| {
        let noisy = &noisy;
        println!("{} greets {x}", noisy.0);
        x + 1
    })
}

fn counter(name: &'static str) -> impl Fn() -> u32 {
    let noisy = Noisy(name);
    move || {
        let noisy = &noisy;
        println!("{} counts", noisy.0);
        2
    }
}

fn call_once(f: Box<dyn FnOnce(u32) -> u32 + '_>) -> u32 {
    f(5)
}

fn main() {
    let greet = greeter("boxed");
    {
        let shared: Rc<dyn Fn(u32) -> u32 + '_> = Rc::new(&*greet);
        let again = Rc::clone(&shared);
        println!("{}", shared(1) + again(2));
    }
    println!("shared dropped");
    {
        let of_box: Rc<dyn Fn(u32) -> u32 + '_> = Rc::new(&greet);
        println!("{}", of_box(3));
    }
    println!("borrowed box dropped");
    println!("{}", call_once(Box::new(&*greet)));
    println!("once dropped");
    let count = counter("returned");
    {
        let boxed: Box<dyn Fn() -> u32 + '_> = Box::new(&count);
        println!("{}", boxed());
    }
    println!("borrowed closure dropped, {}", count());
    let twice: Box<dyn Fn(u32) -> u32> = Box::new(greeter("twice"));
    println!("{}", twice(4));
    drop(twice);
    println!("twice dropped");
    println!("{}", greet(6));
}
