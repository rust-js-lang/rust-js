// What a trait's generic method asks of its own type parameter, its
// `type_name`, is given by its caller, through the trait's dictionary too
// (ADR 0466).

trait Named {
    fn name_of<U>() -> &'static str;

    fn size_of<U: Clone>() -> usize;
}

struct Plain;

impl Named for Plain {
    fn name_of<U>() -> &'static str {
        std::any::type_name::<U>()
    }

    fn size_of<U: Clone>() -> usize {
        std::mem::size_of::<U>()
    }
}

fn through<X: Named>() -> (&'static str, usize) {
    (X::name_of::<u8>(), X::size_of::<u64>())
}

fn main() {
    println!("{:?}", through::<Plain>());
    println!("{}", Plain::name_of::<Vec<u16>>());
}
