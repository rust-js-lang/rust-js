//! Values with a destructor given to each: dropped where Rust drops them.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    let k = dep::keep(Loud(1));
    println!("kept {}", k.0.0);
    let a = dep::pick(Loud(2), Loud(3));
    println!("picked {}", a.0);
    let r = dep::relay(Loud(4));
    println!("relayed {}", r.0.0);
    dep::discard(Loud(5));
    println!("discarded");
    let n = dep::apply(&|l: Loud| l.0 + 1, Loud(6));
    println!("applied {n}");
    let f = dep::pick::<u32, Loud>;
    println!("{}", f(7, Loud(8)));
    let b = dep::keep_by(Loud(9));
    println!("kept by {}", b.0.0);
}
