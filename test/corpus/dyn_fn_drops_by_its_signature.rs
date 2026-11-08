// A `dyn Fn` drops what it holds where a function holding a value with a
// destructor is made one taking what it takes: here a `dyn FnOnce()` of a
// closure holding a `Noisy`, which its call drops. A `dyn Fn(&K)` none
// such is made, held in a generic struct in an `Rc`, drops nothing (#954).
use std::rc::Rc;

struct System<K> {
    notify: Box<dyn Fn(&K)>,
}

fn create<K: 'static>(notify: Box<dyn Fn(&K)>) -> Rc<dyn Fn(&K)> {
    let system = Rc::new(System { notify });
    Rc::new(move |node| (system.notify)(node))
}

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn once(name: &'static str) -> Box<dyn FnOnce()> {
    let noisy = Noisy(name);
    Box::new(move || println!("once {}", noisy.0))
}

fn main() {
    let notify = create(Box::new(|n: &u32| println!("notified {n}")));
    notify(&1);
    notify(&2);
    once("held")();
    println!("done");
}
