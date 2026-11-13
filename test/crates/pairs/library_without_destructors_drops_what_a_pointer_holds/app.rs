//! A crate whose zone's offset has a destructor, held by the library's
//! generic code it reaches through the pointer the library gives.
pub struct Noisy(pub u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub struct Mine;

impl dep::Zone for Mine {
    type Offset = Noisy;
    fn offset(&self) -> Noisy {
        Noisy(7)
    }
}

pub fn main() {
    let held = dep::holder::<Mine>();
    println!("{}", held(&Mine));
    println!("end");
}
