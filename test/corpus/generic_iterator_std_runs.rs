//@ library-differs
// A generic iterator of the crate's that a `for` loop steps, std's code
// running its `next`: what its `next` borrows and drops is of its own
// impl's parameters, which no type at the call gives, so it's not known
// rather than read with the caller's (#240).
struct Chunks<'a, T, const N: usize> {
    rest: &'a [T],
}

impl<'a, T: std::fmt::Debug, const N: usize> Iterator for Chunks<'a, T, N> {
    type Item = [&'a T; N];

    fn next(&mut self) -> Option<[&'a T; N]> {
        if self.rest.len() < N {
            return None;
        }
        let (first, rest) = self.rest.split_at(N);
        self.rest = rest;
        Some(std::array::from_fn(|i| &first[i]))
    }
}

fn chunks<T: std::fmt::Debug, const N: usize>(items: &[T]) -> Chunks<'_, T, N> {
    Chunks { rest: items }
}

fn main() {
    let items = [1, 2, 3, 4, 5, 6, 7];
    for chunk in chunks::<_, 3>(&items) {
        println!("{chunk:?}");
    }
}
