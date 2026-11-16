// An `Rc` of a function item unsized later, `Rc<dyn Fn>`, whose address
// the crate reads, beside generic code that clones a type parameter, `T:
// Clone`, given numbers and text alone: no clone of the `Rc` before it's
// unsized, as nothing gives generic code a type that holds it, so it's a
// function of its own where it's unsized, as alone (#1896). The two cases
// were refused side by side, `dyn_fn_address_by_rc_as_ptr` beside
// `evidence_paths`, by the clone of any `T` anywhere in the crate.
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

fn twice<T: Clone>(x: &T) -> (T, T) {
    (x.clone(), x.clone())
}

trait Getter<T: Clone> {
    fn get(&self) -> T;
}

impl<T: Clone> Getter<T> for Option<T> {
    fn get(&self) -> T {
        self.as_ref().unwrap().clone()
    }
}

fn main() {
    let first: Rc<dyn Fn(u32) -> u32> = Rc::new(double);
    let held = Rc::new(double);
    let later: Rc<dyn Fn(u32) -> u32> = held;
    let a = Rc::as_ptr(&first) as *const ();
    let b = Rc::as_ptr(&later) as *const ();
    println!("{} {}", a == b, first(1) + later(2));
    println!("{:?} {:?}", twice(&3u8), twice(&String::from("ab")));
    println!("{} {}", Some(7).get(), Some("x").get());
}
