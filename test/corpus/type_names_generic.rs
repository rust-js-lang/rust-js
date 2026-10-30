//@ library-differs
// `type_name` in generic code (ADR 0145): of a type that holds a type
// parameter, rustc's name with the one its caller gives in its place; of a
// `T: ?Sized`, as of any; passed on through another generic function; and
// of a blanket impl's own parameter, its method called directly and through
// a `dyn`, whose dictionary is made with it.
use std::any::type_name;
use std::borrow::Cow;

fn names<T: Clone + 'static>() -> (&'static str, &'static str, &'static str) {
    (type_name::<Cow<'static, T>>(), type_name::<Vec<(T, u8)>>(), type_name::<Option<&T>>())
}

fn unsized_name<T: ?Sized>() -> &'static str {
    type_name::<T>()
}

fn passed_on<U: Clone + 'static>() -> &'static str {
    names::<U>().1
}

trait Named {
    fn named(&self) -> &'static str;
}

impl<T: ?Sized> Named for T {
    fn named(&self) -> &'static str {
        type_name::<T>()
    }
}

fn main() {
    println!("{:?}", names::<u32>());
    println!("{:?}", names::<String>());
    println!("{} {}", unsized_name::<str>(), unsized_name::<[u8]>());
    println!("{}", passed_on::<bool>());
    let shown: &dyn Named = &1i32;
    println!("{} {}", shown.named(), 2u8.named());
}
