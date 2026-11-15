//! A library that keeps capacities, as it reads one, so its functions are
//! compiled in it, not in its consumer: their bodies aren't its consumer's
//! to read.
pub fn doubled(it: std::vec::IntoIter<u32>) -> impl Iterator<Item = u32> {
    it.map(|x| x * 2)
}

pub fn room(v: &[u32]) -> usize {
    v.to_vec().capacity()
}
