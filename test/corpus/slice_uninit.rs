// A slice of `MaybeUninit`s is the array of what they hold (ADR 0332):
// written from a slice by copies or clones, read and changed as it is, and
// each item dropped by `assume_init_drop`. And `[const { .. }; N]` of a
// value JS has none of as a constant, a `Vec`'s and a written
// `MaybeUninit`'s: the block's code, a constant of its module, each slot
// its own (ADR 0127).
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
    let mut lists: [Vec<u8>; 2] = [const { Vec::new() }; 2];
    lists[0].push(1);
    let mut held = [const { MaybeUninit::new(String::new()) }; 2];
    held[1].write("h".to_string());
    println!("{lists:?} {:?} {:?}", unsafe { held[0].assume_init_ref() }, unsafe { held[1].assume_init_ref() });
    println!("end");
}
