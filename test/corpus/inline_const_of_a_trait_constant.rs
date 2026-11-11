// A `const { .. }` block of a type parameter's reads its trait constant
// from the dictionary its caller gives, which carries it: `T::V` of
// `const { T::V * 10 + 1 }`, alone, beside a generic associated constant
// and a plain read of it, nested in another, and of a const generic's
// (#1387). The dictionary had no `V`, so it printed 1 where Rust's 21.
trait N {
    const V: u32;
    const NAME: &'static str = "n";
    /// Read only by an inline constant nested in another.
    const SCALE: u32;
}

struct Two;
impl N for Two {
    const V: u32 = 2;
    const SCALE: u32 = 7;
}

struct Three;
impl N for Three {
    const V: u32 = 3;
    const NAME: &'static str = "three";
    const SCALE: u32 = 11;
}

fn inline<T: N>() -> u32 {
    const { T::V * 10 + 1 }
}

fn nested<T: N>() -> u32 {
    const { (const { T::SCALE + 1 }) * 2 }
}

fn named<T: N>() -> &'static str {
    const { T::NAME }
}

fn plain<T: N>() -> u32 {
    T::V * 100
}

fn sized<T: N, const K: usize>() -> usize {
    const { T::V as usize * K }
}

struct W<T>(T);

impl<T: N> W<T> {
    const DOUBLE: u32 = T::V * 2 + 1;
}

fn main() {
    println!("{} {}", inline::<Two>(), W::<Two>::DOUBLE);
    println!("{} {}", inline::<Three>(), plain::<Three>());
    println!("{} {}", nested::<Two>(), nested::<Three>());
    println!("{} {}", named::<Two>(), named::<Three>());
    println!("{} {}", sized::<Two, 4>(), sized::<Three, 5>());
    println!("{}", W::<Three>::DOUBLE);
}
