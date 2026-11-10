// An `Rc` a trait's generic method counts, given by each impl, a default
// and an override, called directly, through a generic caller and from a
// default's own caller: its instances are its namings', the trait's
// method's and each impl's, which `instances_of` follows (#1009, ADR
// 1115).
use std::rc::Rc;

trait Keep {
    fn keep<T>(&self, r: &Rc<T>) -> usize {
        let again = Rc::clone(r);
        Rc::strong_count(&again)
    }
    fn held<T: Clone>(&self, r: &Rc<T>) -> (usize, T) {
        let more = r.clone();
        (Rc::strong_count(r), (*more).clone())
    }
}
struct A;
impl Keep for A {}
struct B;
impl Keep for B {
    fn keep<T>(&self, r: &Rc<T>) -> usize {
        Rc::strong_count(r) * 10
    }
}

fn through<K: Keep, T>(k: &K, r: &Rc<T>) -> usize {
    k.keep(r)
}

fn by_default<K: Keep>(k: &K) -> usize {
    let r = Rc::new(vec![1u8]);
    let _two = r.clone();
    k.keep(&r)
}

fn main() {
    let n = Rc::new(5u32);
    let s = Rc::new(String::from("s"));
    let _s2 = Rc::clone(&s);
    println!("{} {} {} {}", A.keep(&n), A.keep(&s), B.keep(&n), B.keep(&s));
    println!("{} {}", through(&A, &n), through(&B, &s));
    println!("{} {}", by_default(&A), by_default(&B));
    let (c, v) = A.held(&s);
    println!("{c} {v} {}", Rc::strong_count(&s));
    let unit = Rc::new(());
    println!("{} {}", A.keep(&unit), through(&A, &Rc::new(vec![1, 2])));
}
