// A `dyn` of no trait but auto traits, `dyn Send`, is a pair as any `dyn`
// is, whose dictionary is its value's drop alone: dropped where Rust drops
// it, through a `Box`, a `Vec`, an `Rc` and an upcast (rustc's
// drop/rc-trait-object-double-drop.rs).
use std::rc::Rc;

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Named {
    fn name(&self) -> &'static str;
}

impl Named for Noisy {
    fn name(&self) -> &'static str {
        self.0
    }
}

struct Counter<'a>(&'a mut u32);

impl Drop for Counter<'_> {
    fn drop(&mut self) {
        *self.0 += 1;
    }
}

fn keep(_: &dyn Sync) {}

fn main() {
    {
        let _boxed: Box<dyn Send> = Box::new(Noisy("boxed"));
        println!("in scope");
    }
    let many: Vec<Box<dyn Send + Sync>> = vec![Box::new(Noisy("first")), Box::new(5u8), Box::new(Noisy("second"))];
    println!("{} held", many.len());
    drop(many);

    let both: Box<dyn Send + Sync> = Box::new(Noisy("upcast"));
    let one: Box<dyn Sync> = both;
    drop(one);

    let named: Box<dyn Named + Send> = Box::new(Noisy("named"));
    println!("{}", named.name());
    let unnamed: Box<dyn Send> = named;
    drop(unnamed);

    let shared: Rc<dyn Send> = Rc::new(Noisy("shared"));
    let again = shared.clone();
    drop(shared);
    println!("one left");
    drop(again);

    let mut drops = 0;
    {
        let _: Rc<dyn Send> = Rc::new(Counter(&mut drops));
    }
    println!("drops {drops}");

    let kept = Noisy("borrowed");
    keep(&kept);
    println!("end");
}
