// `{:?}` of a `PhantomData<K>` is `PhantomData<{type_name::<K>()}>`; of a
// type parameter, the name its caller gives, as `type_name::<K>()` is, a
// derived `Debug`'s and one written in place (ADR 0452).
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug)]
struct Keyed<K, V> {
    len: usize,
    _key: PhantomData<K>,
    _value: PhantomData<V>,
}

fn shown<K: Debug, V: Debug>(keyed: &Keyed<K, V>) -> String {
    format!("{keyed:?}")
}

fn marker<T>() -> String {
    format!("{:?}", PhantomData::<T>)
}

fn main() {
    let keyed: Keyed<u32, String> = Keyed {
        len: 2,
        _key: PhantomData,
        _value: PhantomData,
    };
    println!("{keyed:?}");
    println!("{}", shown(&Keyed::<String, Vec<u8>> { len: 1, _key: PhantomData, _value: PhantomData }));
    println!("{}", marker::<Option<i64>>());
}
