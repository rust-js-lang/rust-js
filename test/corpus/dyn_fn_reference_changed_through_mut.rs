// A `&dyn Fn` variable given by `&mut` to a function that points it at
// another closure: the call is given a box of the variable, and the
// variable calls the closure the function set (#460). Once rust-js
// panicked asking the layout of the parameter's `&dyn Fn`, whose lifetime
// the function's signature binds.
fn other(g: &mut &dyn Fn() -> u32) {
    *g = &|| 4;
}

fn kept(g: &mut &dyn Fn() -> u32) -> u32 {
    g() + 1
}

pub fn call(f: &dyn Fn() -> u32) -> u32 {
    let mut g = f;
    let before = kept(&mut g);
    other(&mut g);
    before * 10 + g()
}

fn main() {
    let three = || 3;
    println!("{}", call(&three));
    let base = 7;
    let seven = move || base;
    println!("{}", call(&seven));
}
