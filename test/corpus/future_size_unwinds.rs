// A future's state is laid out as rust-js's panics are: they unwind, as
// a JS exception does (ADR 0090). A local that lives across a call that may
// panic, and has a destructor to run then, is kept in it, so `single_with_noop`'s is a byte bigger than
// `single`'s, as natively, where wasm32's abort would make them the same.
struct Big([u8; 64]);

impl Drop for Big {
    fn drop(&mut self) {}
}

fn noop() {}
async fn fut() {}

async fn single() {
    let x;
    fut().await;
    x = Big([0; 64]);
}

async fn single_with_noop() {
    let x;
    fut().await;
    noop();
    x = Big([0; 64]);
    noop();
}

fn main() {
    let a = std::mem::size_of_val(&single());
    let b = std::mem::size_of_val(&single_with_noop());
    println!("{a} {b} {}", cfg!(panic = "unwind"));
}
