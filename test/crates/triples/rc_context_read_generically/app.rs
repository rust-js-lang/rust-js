// React DOM's bindings, of both: a context `dep` makes, read by `mid`.
pub fn main() {
    let theme = dep::create_context(Some("light"));
    println!("{:?}", mid::read_context(&theme));
    mid::push_context(&theme, Some("dark"));
    println!("{:?}", mid::read_context(&theme));
    println!("{}", mid::pop_context(&theme));
    println!("{:?}", mid::read_context(&theme));
    let count = dep::create_context(7u32);
    println!("{}", mid::read_context(&count));
}
