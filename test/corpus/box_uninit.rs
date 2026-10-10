// A `Box` made before its value: `new_uninit` and `Box::write`, a slice's
// slots written one by one, `new_zeroed` of numbers, and `assume_init`.

use std::mem::MaybeUninit;

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let slot = Box::<u32>::new_uninit();
    let five = Box::write(slot, 5);
    println!("{five}");
    let mut point = Box::<Point>::new_uninit();
    let written: &mut Point = point.write(Point { x: 1, y: 2 });
    written.x += 10;
    let point = unsafe { point.assume_init() };
    println!("{point:?}");
    let mut squares = Box::<[u64]>::new_uninit_slice(4);
    for (i, slot) in squares.iter_mut().enumerate() {
        slot.write((i * i) as u64);
    }
    let squares = unsafe { squares.assume_init() };
    println!("{squares:?}");
    let zero = unsafe { Box::<(u8, f64, bool)>::new_zeroed().assume_init() };
    println!("{zero:?}");
    let zeros = unsafe { Box::<[i16]>::new_zeroed_slice(3).assume_init() };
    println!("{zeros:?}");
    let mut text: MaybeUninit<String> = MaybeUninit::uninit();
    let written: &mut String = text.write(String::from("t"));
    written.push('!');
    println!("{}", unsafe { text.assume_init() });
    let empty = Some(MaybeUninit::<u8>::uninit());
    println!("{:?} {}", empty, empty.is_some());
}
