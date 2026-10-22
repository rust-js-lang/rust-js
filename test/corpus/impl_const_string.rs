// A trait impl's constant whose value rustc's value tree can't say, a
// `String`'s or a `Vec`'s, is its initializer, lowered in its dictionary.
trait Monoid {
    const EMPTY: Self;
    fn combine(&self, other: &Self) -> Self;
}

impl Monoid for String {
    const EMPTY: String = String::new();
    fn combine(&self, other: &String) -> String {
        format!("{self}{other}")
    }
}

impl Monoid for Vec<u8> {
    const EMPTY: Vec<u8> = Vec::new();
    fn combine(&self, other: &Vec<u8>) -> Vec<u8> {
        [self.as_slice(), other].concat()
    }
}

fn fold_all<M: Monoid>(xs: &[M]) -> M {
    xs.iter().fold(M::EMPTY, |a, b| a.combine(b))
}

fn main() {
    println!("{}", fold_all(&["a".to_string(), "bc".to_string()]));
    println!("{:?}", fold_all(&[vec![1u8], vec![2, 3]]));
    let mut empty = String::EMPTY;
    empty.push('x');
    println!("{empty} {:?}", String::EMPTY);
}
