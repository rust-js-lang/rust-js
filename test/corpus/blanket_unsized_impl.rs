//@ library-differs
// A blanket impl over `?Sized` types, `impl<T: Iterator + ?Sized> Ext for
// T {}`, as itertools' `Itertools` is: each provided method it keeps, the
// trait's default, in its dictionary, called of an iterator or of a type
// parameter that may be unsized (ADR 0536).
trait Ext: Iterator {
    fn second(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.next();
        self.next()
    }

    fn skip_one(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

impl<T> Ext for T where T: Iterator + ?Sized {}

fn first_two<I: Iterator<Item = u32> + ?Sized>(it: &mut I) -> (Option<u32>, Option<u32>) {
    (it.skip_one(), it.next())
}

fn main() {
    println!("{:?}", vec![1, 2, 3].into_iter().second());
    let mut range = 1..5;
    range.skip_one();
    println!("{:?}", range.second());
    println!("{:?}", first_two(&mut (7..9)));
    println!("{:?}", first_two(&mut vec![4u32].into_iter()));
}
