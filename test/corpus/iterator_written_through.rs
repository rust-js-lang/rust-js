// `*b = b_orig.clone()` of a `&mut J` to a type parameter given as it is
// (ADR 0411), its caller's own value, an object: written in place, as a
// `&mut` to any object is, `$assign`, as itertools' `Product` restarts its
// inner iterator (ADR 0481).
#[derive(Clone, Debug)]
struct Count(u32, u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            self.0
        })
    }
}

struct Product<I: Iterator, J> {
    a: I,
    a_cur: Option<I::Item>,
    b: J,
    b_orig: J,
}

impl<I, J> Iterator for Product<I, J>
where
    I: Iterator,
    J: Clone + Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, J::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let Self { a, a_cur, b, b_orig } = self;
        let elt_b = match b.next() {
            None => {
                *b = b_orig.clone();
                match b.next() {
                    None => return None,
                    Some(x) => {
                        *a_cur = a.next();
                        x
                    }
                }
            }
            Some(x) => x,
        };
        a_cur.clone().map(|a| (a, elt_b))
    }
}

fn product<I: Iterator, J: Clone + Iterator>(mut a: I, b: J) -> Product<I, J>
where
    I::Item: Clone,
{
    Product { a_cur: a.next(), a, b: b.clone(), b_orig: b }
}

fn main() {
    println!("{:?}", product(1..3, Count(0, 2)).collect::<Vec<_>>());
    println!("{:?}", product(vec!['a', 'b'].into_iter(), vec![7, 8].into_iter()).collect::<Vec<_>>());
}
