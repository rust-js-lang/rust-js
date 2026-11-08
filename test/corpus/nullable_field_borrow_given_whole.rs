// A `&mut` to a nullable field given whole, `Box::new(&mut fiber.owner)`,
// then written through: a handle on the first handle's `value`, read as the
// field, which stores `null` for `None`, as the first one stores, where
// tidy dropped the first while the second still wrote through it (#982).
struct Fiber {
    #[cfg_attr(rust_js, rust_js::nullable)]
    owner: Option<u32>,
}

fn none_of(n: u32) -> Option<u32> {
    if n > 3 { Some(n) } else { None }
}

fn main() {
    let mut fiber = Fiber { owner: Some(3) };
    let slot = Box::new(&mut fiber.owner);
    **slot = none_of(1);
    println!("{:?}", fiber.owner);
    let mut boxed = Box::new(Fiber { owner: None });
    let held = Box::new(&mut boxed.owner);
    **held = none_of(7);
    println!("{:?}", boxed.owner);
}
