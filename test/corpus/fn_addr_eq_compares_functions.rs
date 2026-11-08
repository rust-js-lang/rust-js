// `ptr::fn_addr_eq(f, g)`, what rustc's `unpredictable_function_pointer_
// comparisons` lint asks for in place of `f == g`, compares two function
// pointers as `==` does, by the function, `===` (ADR 0395, #987): a
// function against itself, one against another, a closure's pointer, and a
// generic function's instance its module knows whole (#974).
use std::ptr::fn_addr_eq;

fn double(x: i32) -> i32 {
    x * 2
}

fn triple(x: i32) -> i32 {
    x * 3
}

fn widened<T: Into<i64>>(x: T) -> i64 {
    x.into()
}

fn pick(by_two: bool) -> fn(i32) -> i32 {
    if by_two { double } else { triple }
}

fn main() {
    let ops: [fn(i32) -> i32; 3] = [double, triple, |x| x + 1];
    let found = ops.iter().position(|&op| fn_addr_eq(op, pick(false)));
    println!("{found:?} {}", ops[found.unwrap()](5));
    println!(
        "{} {}",
        fn_addr_eq(pick(true), double as fn(i32) -> i32),
        fn_addr_eq(pick(true), pick(false))
    );
    let inc = ops[2];
    println!("{} {}", fn_addr_eq(inc, ops[2]), fn_addr_eq(inc, ops[0]));
    let wide: fn(u8) -> i64 = widened::<u8>;
    println!("{} {}", fn_addr_eq(wide, widened::<u8> as fn(u8) -> i64), wide(7));
}
