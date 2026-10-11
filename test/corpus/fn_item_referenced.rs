// A function given by reference, `&hello` where a `&dyn Fn()` is taken:
// the function, as a reference is what it refers to.
fn hello() {
    println!("hello");
}

fn call(f: &dyn Fn()) {
    f();
}

fn main() {
    call(&hello);
}
