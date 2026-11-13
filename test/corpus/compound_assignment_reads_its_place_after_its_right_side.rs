// `x += { x = n; 1 }`: Rust evaluates a built-in compound assignment's right
// side first, so the add reads the `x` the block wrote, and wraps at
// `u32::MAX` as native Rust's does (#1660).
fn bump(n: u32) -> u32 {
    let mut x: u32 = 0;
    x += {
        x = n;
        1
    };
    x
}

fn main() {
    println!("{}", bump(u32::MAX));
    println!("{}", bump(7));
}
