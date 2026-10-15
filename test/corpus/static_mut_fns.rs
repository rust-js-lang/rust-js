// A `static mut` whose value rustc can't say, a function's, is its
// initializer's, in its `{ value }` (ADR 0096): one module's own is a `let`,
// one another may write its cell.
fn double(x: i32) -> i32 {
    2 * x
}

fn square(x: i32) -> i32 {
    x * x
}

struct Handler {
    run: fn(i32) -> i32,
    name: &'static str,
}

static mut CURRENT: fn(i32) -> i32 = double;
pub static mut HANDLER: Handler = Handler { run: double, name: "double" };
static mut TABLE: [fn(i32) -> i32; 2] = [double, square];

// One whose value is changed in place, which a `static`'s couldn't be.
#[allow(dead_code)]
struct Registry {
    names: Vec<&'static str>,
    fallback: fn(i32) -> i32,
}

static mut REGISTRY: Registry = Registry { names: Vec::new(), fallback: double };

fn main() {
    unsafe {
        let before = CURRENT(5);
        CURRENT = square;
        println!("{before} {}", CURRENT(5));
        let (name, run) = (HANDLER.name, HANDLER.run);
        println!("{name} {}", run(3));
        HANDLER = Handler { run: square, name: "square" };
        let (name, run) = (HANDLER.name, HANDLER.run);
        println!("{name} {}", run(3));
        TABLE[0] = square;
        let (first, second) = (TABLE[0](4), TABLE[1](4));
        println!("{first} {second}");
        REGISTRY = Registry { names: vec!["square"], fallback: square };
        let fallback = REGISTRY.fallback;
        println!("{}", fallback(6));
    }
}
