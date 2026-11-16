// A `&mut dyn` made of a whole local, `fmt::write(&mut s, ..)`'s `String`
// and `run(&mut b)`'s `Box<dyn FnMut>`, in a crate that asks an address, a
// raw pointer's `offset_from`: the pair as it is, as the crate gives it
// without the address, not a pointer to a variable JS holds as it is,
// which was refused (#1893).
use std::fmt;

fn run(f: &mut dyn FnMut(i32)) {
    f(3);
}

fn main() {
    let items = [10u32, 20, 30];
    let p = items.as_ptr();
    let q = &items[2] as *const u32;
    println!("{}", unsafe { q.offset_from(p) });

    let mut s = String::new();
    fmt::write(&mut s, format_args!("{}-{}", 3, "x")).unwrap();
    println!("{s}");

    let mut sum = 0;
    let mut b: Box<dyn FnMut(i32)> = Box::new(|x| sum += x);
    run(&mut b);
    run(&mut b);
    drop(b);
    println!("{sum}");
}
