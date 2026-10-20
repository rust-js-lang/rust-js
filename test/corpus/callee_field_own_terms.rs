// A generic function's field of its type parameter's associated type,
// `Option<I::Item>`, called where that type has a name of the caller's,
// `I: Iterator<Item = A>`: its fields are read in the function's own terms.
#[derive(Clone)]
struct Pairs<I: Iterator> {
    first: Option<I::Item>,
    rest: I,
}

impl<I: Iterator + Clone> From<I> for Pairs<I> {
    fn from(mut rest: I) -> Self {
        Pairs {
            first: rest.next(),
            rest,
        }
    }
}

impl<I, A> Iterator for Pairs<I>
where
    I: Iterator<Item = A> + Clone,
    A: Clone,
{
    type Item = (A, A);

    fn next(&mut self) -> Option<(A, A)> {
        let first = self.first.clone()?;
        match self.rest.next() {
            Some(second) => Some((first, second)),
            None => {
                let again: Pairs<I> = self.rest.clone().into();
                self.first = again.first;
                None
            }
        }
    }
}

fn main() {
    let pairs: Pairs<std::vec::IntoIter<u32>> = vec![1, 2, 3].into_iter().into();
    let all: Vec<(u32, u32)> = pairs.collect();
    println!("{all:?}");
}
