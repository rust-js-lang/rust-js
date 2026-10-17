// A borrow of a temporary, `&mut 42`, is the box it's made in, which a raw
// pointer of it is a handle on: what it reads stays the temporary's, and
// writing through one changes only it (rustc's
// coercion/basic-ptr-coercions.rs).
#[allow(unused)]
fn unused() {
    let x: &mut isize = &mut 42;
    let _x: &isize = x;
    let _x: &isize = &mut 42;
    let x: &isize = &42;
    let _x: *const isize = x;
    let _x: *const isize = &42;
    let x: &mut isize = &mut 42;
    let _x: *const isize = x;
    let _x: *const isize = &mut 42;
    let _x: *mut isize = &mut 42;
    let _x: *const isize = x;
}

fn main() {
    unused();
    let x: &mut isize = &mut 42;
    let a: &isize = x;
    println!("{a}");
    let b: *const isize = &mut 43;
    let c: *mut isize = &mut 44;
    unsafe {
        *c += 1;
        println!("{} {}", *b, *c);
    }
}
