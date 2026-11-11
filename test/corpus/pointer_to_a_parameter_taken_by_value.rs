// A raw pointer to a parameter taken by value, `&raw mut n`: the parameter
// is a cell of the function's own, made of what it's given, as a local one
// a pointer is taken to is (ADR 0407, #1381). Its callers give the value,
// which the function's writes through the pointer never change.
#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
    y: i32,
}

fn set(mut n: u32) -> u32 {
    let p = &raw mut n;
    unsafe { *p = 7 };
    n
}

fn flip(mut b: bool) -> bool {
    let p = &raw mut b;
    unsafe { *p = !*p };
    b
}

fn moved(mut point: Point, by: i32) -> Point {
    let p = &raw mut point.x;
    unsafe { *p += by };
    point
}

fn read_only(n: u64) -> u64 {
    let p = &raw const n;
    unsafe { *p * 2 }
}

fn main() {
    let given = 1;
    println!("{} {given}", set(given));
    println!("{} {}", flip(true), flip(false));
    let start = Point { x: 1, y: 2 };
    println!("{:?} {start:?}", moved(start, 10));
    println!("{}", read_only(21));
}
