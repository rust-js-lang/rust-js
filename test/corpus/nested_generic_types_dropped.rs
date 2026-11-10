// `nested_generic_types.rs`' types whose parts double at each level, the
// innermost of them with a `Drop` of its own: each call of that `drop` is
// checked for what it's given, which asks what each type argument drops but
// its type parameters. That walk is kept once for each type too, as the
// plain one is, or compiling this took two minutes (#1214, found by the
// metamorphic net's `loud` rewrite of `nested_generic_types.rs`);
// `test/compile-bounds.test.ts` holds it under a bound.
struct S0<T>(T, T);
struct S1<T>(Option<Box<S0<S0<T>>>>, Option<Box<S0<S0<T>>>>);
struct S2<T>(Option<Box<S1<S1<T>>>>, Option<Box<S1<S1<T>>>>);
struct S3<T>(Option<Box<S2<S2<T>>>>, Option<Box<S2<S2<T>>>>);
struct S4<T>(Option<Box<S3<S3<T>>>>, Option<Box<S3<S3<T>>>>, Option<T>);

impl<T> Drop for S0<T> {
    fn drop(&mut self) {
        println!("drop S0");
    }
}

fn main() {
    let held = S4::<u8>(None, None, Some(1));
    let pair = S0(S4::<u8>(None, None, Some(2)), S4::<u8>(None, None, None));
    println!("{} {:?} {:?}", held.2.is_some(), pair.0.2, pair.1.2);
}
