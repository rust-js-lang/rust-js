//! A library's generic function that gives its parameter's iterator,
//! adapted, which its consumer compiles here, as a library that keeps no
//! capacities has each function the consumer reaches (#1180).
pub fn doubled<I: Iterator<Item = u32>>(it: I) -> impl Iterator<Item = u32> {
    it.map(|x| x * 2)
}
