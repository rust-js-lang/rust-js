//! A zone of this crate's whose offset has a destructor, held by the
//! library's generic code.
struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped");
    }
}

struct Here;

impl dep::Zone for Here {
    type Offset = Loud;
    fn offset(&self) -> Loud {
        Loud
    }
}

pub fn main() {
    println!("{}", dep::hold(&Here));
    println!("end");
}
