// A slice's raw pointer cast to one of what's at the same place, `*mut
// [MaybeUninit<T>]` as `*mut [T]`, as itertools' `slice_assume_init_mut`
// does: the slice itself, as a slice's pointer is (ADRs 0332, 0407, 0482).
use std::mem::MaybeUninit;

unsafe fn slice_assume_init_mut<T>(slice: &mut [MaybeUninit<T>]) -> &mut [T] {
    unsafe { &mut *(slice as *mut [MaybeUninit<T>] as *mut [T]) }
}

fn main() {
    let mut buf: [MaybeUninit<u32>; 3] = [MaybeUninit::new(1), MaybeUninit::new(2), MaybeUninit::new(3)];
    let all = unsafe { slice_assume_init_mut(&mut buf) };
    all[1] = 20;
    println!("{:?}", all);
    let mut words: [MaybeUninit<String>; 2] = [MaybeUninit::new("a".to_string()), MaybeUninit::new("b".to_string())];
    let ws = unsafe { slice_assume_init_mut(&mut words) };
    ws[0].push('!');
    println!("{:?}", ws);
    unsafe { std::ptr::drop_in_place(ws) };
}
