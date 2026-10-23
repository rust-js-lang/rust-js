// A closure that holds a value with a destructor made as a block's value,
// for a let: what it took dropped where the closure is.
use std::rc::Rc;
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let r = Rc::new(5);
    let c = {
        let r = r.clone();
        move || *r
    };
    println!("{} {}", c(), Rc::strong_count(&r));
    drop(c);
    println!("{}", Rc::strong_count(&r));
    let f = {
        let d = D("in block");
        println!("making");
        move || d.0.len()
    };
    println!("call {}", f());
    println!("end");
}
