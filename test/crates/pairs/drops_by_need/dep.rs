//! Generic functions given a consumer's values, each given a drop only for
//! what its body may drop (ADR 0300).
pub struct Kept<T>(pub T);

/// Moves its value before anything can leave: it drops none.
pub fn keep<T>(value: T) -> Kept<T> {
    Kept(value)
}

/// Gives back `a` and drops `b`.
pub fn pick<A, B>(a: A, _b: B) -> A {
    a
}

/// Passes its value on to `keep`, which drops none.
pub fn relay<T>(value: T) -> Kept<T> {
    keep(value)
}

/// Passes its value on to `pick`, as the one it drops.
pub fn discard<T>(value: T) {
    pick((), value);
}

/// Calls `f` with its value, which the call moves.
pub fn apply<A, R>(f: &dyn Fn(A) -> R, a: A) -> R {
    f(a)
}

/// Gives `keep` itself to `apply`: the function, with no drop to bind.
pub fn keep_by<T>(value: T) -> Kept<T> {
    apply(&keep, value)
}
