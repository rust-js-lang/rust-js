// A `#[rust_js::nullable]` place written through a borrow beside a
// wrapper: a nullable parameter swapped with what a struct holds,
// `mem::swap(&mut x, &mut other.0)`, each side given what's written to it
// so, `x = other[0] ?? null`, where a destructuring wrote `undefined`;
// and a `&mut` to a nullable field kept in a struct, `Held(&mut
// fiber.0.owner)`, the handle that stores `null` for `None` given on,
// where a handle on its `value` was made that stored it as given (#1018).
struct Held<T>(T);

fn none_of(n: u32) -> Option<u32> {
    if n > 3 { Some(n) } else { None }
}

#[cfg_attr(rust_js, rust_js::nullable(x))]
fn absent(x: Option<u32>) -> bool {
    x.is_none()
}

#[cfg_attr(rust_js, rust_js::nullable(x))]
fn param_swapped(mut x: Option<u32>) -> bool {
    let mut other = Held(none_of(1));
    std::mem::swap(&mut x, &mut other.0);
    absent(x) && other.0 == Some(3)
}

struct Fiber {
    #[cfg_attr(rust_js, rust_js::nullable)]
    owner: Option<u32>,
}

fn report(f: &Fiber) -> bool {
    absent(f.owner)
}

fn main() {
    println!("{}", param_swapped(Some(3)));
    let mut kept = Fiber { owner: None };
    let mut held = Held(Some(5));
    std::mem::swap(&mut kept.owner, &mut held.0);
    std::mem::swap(&mut kept.owner, &mut held.0);
    println!("{} {:?}", report(&kept), held.0);
    let mut fiber = Held(Fiber { owner: Some(3) });
    let slot = Held(&mut fiber.0.owner);
    *slot.0 = none_of(1);
    println!("{}", report(&fiber.0));
    let mut plain = Fiber { owner: Some(3) };
    let slot = &mut plain.owner;
    *slot = none_of(1);
    println!("{}", report(&plain));
}
