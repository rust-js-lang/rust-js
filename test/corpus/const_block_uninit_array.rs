// `[const { MaybeUninit::uninit() }; N]` of a type that isn't `Copy` (ADR
// 0332), and a named constant's: each slot the constant's value, nothing
// yet, `undefined`, which needs no copy where the array is written; then
// written, and read back.
use std::mem::MaybeUninit;
use std::ptr;

/// A vector's items moved out into an array of slots, as smallvec's
/// `from_vec` does with its inline one.
fn take_all(mut v: Vec<String>) -> Vec<String> {
    let mut moved: [MaybeUninit<String>; 3] = [const { MaybeUninit::uninit() }; 3];
    let len = v.len();
    unsafe {
        v.set_len(0);
        ptr::copy_nonoverlapping(v.as_ptr(), moved.as_mut_ptr() as *mut String, len);
    }
    moved[..len].iter().map(|slot| unsafe { slot.assume_init_read() }).collect()
}

const EMPTY: MaybeUninit<String> = MaybeUninit::uninit();

fn main() {
    println!("{:?}", take_all(vec!["a".to_string(), "b".to_string()]));
    let mut lists: [MaybeUninit<Vec<u8>>; 2] = [const { MaybeUninit::uninit() }; 2];
    for (i, slot) in lists.iter_mut().enumerate() {
        slot.write(vec![i as u8; i + 1]);
    }
    let lists = lists.map(|slot| unsafe { slot.assume_init() });
    println!("{lists:?}");
    let mut named = [EMPTY; 2];
    named[1].write("n".to_string());
    println!("{}", unsafe { named[1].assume_init_ref() });
}
