//! A library's trait whose generic method its generic code calls with a
//! `Vec`, which a consumer's impl of it can't see: the consumer's impl takes
//! its `Clone` dictionary, the trait's callers unseen (#1233).
pub trait Visit {
    fn twin<T: Clone>(&self, t: &T) -> (T, T);
}

pub fn run<V: Visit>(v: &V) -> String {
    let source = vec![1, 2];
    let (mut a, b) = v.twin(&source);
    a.push(3);
    format!("{source:?} {a:?} {b:?}")
}
