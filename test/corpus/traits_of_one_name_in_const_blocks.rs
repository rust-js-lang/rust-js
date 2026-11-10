// Two traits of one name, each a `const _` block's, and an impl of each for
// one type: zerocopy's `impl_or_verify!` and `static_assert!` write them,
// a block for each, all in one module. Their dictionaries' names are
// numbered, `u16_Subtrait` and `u16_Subtrait2`, as two types of one name
// local to two functions are; the second was refused, "generated trait
// implementation name collides" (#1298).
trait Marker {
    fn name() -> &'static str;
}

impl Marker for u16 {
    fn name() -> &'static str {
        "u16"
    }
}

const _: () = {
    trait Subtrait: Marker {}
    impl Subtrait for u16 {}
};

const _: () = {
    trait Subtrait: Marker {}
    impl Subtrait for u16 {}
};

fn main() {
    println!("{}", u16::name());
}
