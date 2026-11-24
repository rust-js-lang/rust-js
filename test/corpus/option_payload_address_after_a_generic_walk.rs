//@ library-differs
// An `Option` payload's address in a tuple, read after generic code has
// walked the tuple's fields by reference: where generic code, which knows
// no layout, made the field's handle first, the caller's offset is still
// the place's, so the payload is in the tuple's allocation, 4 bytes in,
// not an allocation of its own (#2543).
trait Observe {
    fn seen(&self) {}
}

impl Observe for u8 {}
impl Observe for P {}

impl<T: Observe> Observe for Option<T> {
    fn seen(&self) {
        if let Some(x) = self {
            x.seen();
        }
    }
}

impl<A: Observe, B: Observe> Observe for (A, B) {
    fn seen(&self) {
        self.0.seen();
        self.1.seen();
    }
}

fn look<T: ?Sized + Observe>(x: &T) {
    x.seen();
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct P {
    a: u8,
    b: u32,
}

fn main() {
    let pair = (Some(P { a: 7, b: 8 }), 9u8);
    look(&pair);
    let pb = &raw const pair as usize;
    println!("in a tuple {}", pair.0.as_ref().unwrap() as *const P as usize - pb);
    println!("{:?}", pair.0);
}
