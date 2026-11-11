// A raw pointer to what a `&mut` refers to, in a function whose result
// reborrows it, which its callers give a handle on its place for: httparse's
// `deinit_slice_mut`, a `&mut &mut [T]` read as a `&mut &mut
// [MaybeUninit<T>]`, the same view, filled and shortened through it, and
// `assume_init_slice` back (#1283).
use std::mem::MaybeUninit;

unsafe fn deinit_slice_mut<'a, 'b, T>(s: &'a mut &'b mut [T]) -> &'a mut &'b mut [MaybeUninit<T>] {
    let s: *mut &mut [T] = s;
    let s = s as *mut &mut [MaybeUninit<T>];
    unsafe { &mut *s }
}

unsafe fn assume_init_slice<T>(s: &mut [MaybeUninit<T>]) -> &mut [T] {
    let s: *mut [MaybeUninit<T>] = s;
    let s = s as *mut [T];
    unsafe { &mut *s }
}

fn fill(headers: &mut &mut [MaybeUninit<u32>]) -> usize {
    let mut n = 0;
    for (i, slot) in headers.iter_mut().enumerate().take(2) {
        slot.write(i as u32 * 10 + 1);
        n += 1;
    }
    let taken = std::mem::take(headers);
    *headers = &mut taken[..n];
    n
}

fn main() {
    let mut storage = [0u32; 4];
    let mut headers: &mut [u32] = &mut storage;
    let n = fill(unsafe { deinit_slice_mut(&mut headers) });
    println!("{n} {:?}", headers);
    let mut raw = [MaybeUninit::new(5u32), MaybeUninit::new(6)];
    let init = unsafe { assume_init_slice(&mut raw) };
    init[1] = 60;
    println!("{:?}", init);
    println!("{:?}", storage);
}
