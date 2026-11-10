//@ native: wasm32
// `ptr::swap_nonoverlapping(a, b, n)`: each of the `n` items from where each
// points, the other's, `$ptrSwap(a, b, n)`, as `ptr::swap`'s is of one:
// halves of one array, of a `Vec` and a boxed array, of `String`s, in
// generic code, of one place each, and bytes of two words through their
// byte views, as std swaps their bytes (#1256).
use std::ptr;

fn halves_swapped<T>(items: &mut [T], n: usize) {
    let half = items.len() / 2;
    let p = items.as_mut_ptr();
    unsafe { ptr::swap_nonoverlapping(p, p.add(half), n) };
}

fn main() {
    let mut a = [1u32, 2, 3, 4];
    let p = a.as_mut_ptr();
    unsafe { ptr::swap_nonoverlapping(p, p.add(2), 2) };
    println!("{a:?}");
    let mut b = vec![10u16, 20, 30];
    let mut c = Box::new([7u16, 8, 9]);
    unsafe { ptr::swap_nonoverlapping(b.as_mut_ptr().add(1), c.as_mut_ptr(), 2) };
    println!("{b:?} {c:?}");
    let mut names = vec![
        String::from("a"),
        String::from("b"),
        String::from("c"),
        String::from("d"),
    ];
    halves_swapped(&mut names, 2);
    println!("{names:?}");
    let mut words = [1u64, 2, 3, 4, 5];
    halves_swapped(&mut words, 1);
    halves_swapped(&mut words, 0);
    println!("{words:?}");
    let mut x = String::from("x");
    let mut y = String::from("y");
    unsafe { ptr::swap_nonoverlapping(&mut x, &mut y, 1) };
    println!("{x} {y}");
    let mut d = [0x1122_3344u32, 0x5566_7788];
    let bytes = d.as_mut_ptr().cast::<u8>();
    unsafe { ptr::swap_nonoverlapping(bytes.add(1), bytes.add(5), 2) };
    println!("{d:x?}");
}
