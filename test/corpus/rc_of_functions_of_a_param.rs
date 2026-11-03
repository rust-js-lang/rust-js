// A generic struct in an `Rc` whose type parameter only its functions'
// signatures name, `Box<dyn Fn(&K)>`, drops no value of it: a closure that
// captures the `Rc` holds no `K`, though the crate has a `Drop` (ADR 0521,
// amended).
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

fn main() {
    let notify = create(Box::new(|n: &u32| println!("notified {n}")));
    notify(&1);
    notify(&2);
    {
        let _noisy = Noisy("scope");
        notify(&3);
    }
    println!("done");
}
