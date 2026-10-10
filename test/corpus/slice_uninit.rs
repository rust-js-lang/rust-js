// A slice of `MaybeUninit`s is the array of what they hold (ADR 0332):
// written from a slice by copies or clones, read and changed as it is, and
// each item dropped by `assume_init_drop`.
use std::mem::MaybeUninit;

struct Noisy(i32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Clone for Noisy {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Noisy(self.0 + 10)
    }
}

fn main() {
    let mut buf = [MaybeUninit::<i32>::uninit(); 3];
    let written = buf.write_copy_of_slice(&[1, 2, 3]);
    written[0] = 9;
    println!("{:?}", unsafe { buf.assume_init_ref() });
    unsafe {
        buf.assume_init_mut()[2] = 7;
    }
    println!("{:?}", unsafe { buf.assume_init_ref() });

    let src = [Noisy(1), Noisy(2)];
    let mut slots = [const { MaybeUninit::<Noisy>::uninit() }; 2];
    let cloned = slots.write_clone_of_slice(&src);
    println!("{}", cloned[1].0);
    unsafe {
        slots.assume_init_drop();
    }
    println!("end");
}
