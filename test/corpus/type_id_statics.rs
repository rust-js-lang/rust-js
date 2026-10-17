// A `TypeId` in a static or a constant is its type's key, a string, which
// every use shares (ADR 0331; rustc's consts/const-typeid-of-rpass.rs).
use std::any::TypeId;

struct A;

static ID_ISIZE: TypeId = TypeId::of::<isize>();

const IDS: (TypeId, TypeId, TypeId) = (TypeId::of::<usize>(), TypeId::of::<&'static str>(), TypeId::of::<A>());

fn main() {
    println!("{}", ID_ISIZE == TypeId::of::<isize>());
    println!("{}", ID_ISIZE == TypeId::of::<usize>());
    println!("{} {} {}", IDS.0 != IDS.1, IDS.0 == TypeId::of::<usize>(), IDS.2 == TypeId::of::<A>());
    const F: (TypeId, TypeId) = (TypeId::of::<fn(fn(A) -> A) -> A>(), TypeId::of::<fn(fn() -> A, A) -> A>());
    println!("{}", F.0 != F.1);
}
