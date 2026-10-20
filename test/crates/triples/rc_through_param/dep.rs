// A library with no `Rc` in sight, given one by a type parameter: its clone
// and its drop count it (ADR 0523).
pub fn pair<T: Clone>(t: T) -> (T, T) {
    (t.clone(), t)
}

pub fn count_all<T>(items: Vec<T>) -> usize {
    items.len()
}
