// vec![x; n] of a value with a destructor: n - 1 clones and x itself, as
// std's from_elem makes them, or x dropped of none.
use std::rc::Rc;
#[derive(Clone)]
struct D(u8);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let r = Rc::new(5);
    let v = vec![r.clone(); 3];
    println!("{} {}", v.len(), Rc::strong_count(&r));
    drop(v);
    println!("{}", Rc::strong_count(&r));
    let none: Vec<Rc<i32>> = vec![r.clone(); 0];
    println!("{} {}", none.len(), Rc::strong_count(&r));
    let ds = vec![D(7); 2];
    println!("made {}", ds.len());
    let zero = vec![D(9); 0];
    println!("zero {}", zero.len());
    let n = std::hint::black_box(3);
    let grid = vec![vec![D(1); 1]; n];
    println!("grid {}", grid.len());
}
