//@ library-differs
struct Word(Vec<char>);

impl<'a> IntoIterator for &'a Word {
    type Item = &'a char;
    type IntoIter = std::slice::Iter<'a, char>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

fn letters<'a, I>(r: &'a I) -> Vec<&'a char>
where
    &'a I: IntoIterator<Item = &'a char>,
{
    Vec::from_iter(r)
}

fn sizes<'a, I>(r: &'a I) -> (Vec<char>, String)
where
    &'a I: IntoIterator<Item = &'a char>,
{
    (Vec::from_iter(r.into_iter().copied()), String::from_iter(r))
}

fn main() {
    let w = Word(vec!['a', 'b', 'c']);
    println!("{:?}", letters(&w));
    println!("{:?}", sizes(&w));
    let v = vec!['x', 'y'];
    println!("{:?}", letters(&v));
}
