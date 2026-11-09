//! `fold_with`, which its consumer compiles itself as it's given closures
//! that drop, makes `joined`'s closure, which rustc inlined into it: placed
//! where `fold_with` is, not asked of `joined`, which no module of the
//! consumer's holds (#1029).
pub fn joined<T, B, Acc>(mut f: impl FnMut(T) -> B, mut g: impl FnMut(Acc, B) -> Acc) -> impl FnMut(Acc, T) -> Acc {
    move |acc, x| g(acc, f(x))
}

pub fn fold_with<I: Iterator<Item = i32>, F: FnMut(i32) -> i32, Acc, G: FnMut(Acc, i32) -> Acc>(
    it: I,
    f: F,
    init: Acc,
    g: G,
) -> Acc {
    it.fold(init, joined(f, g))
}
