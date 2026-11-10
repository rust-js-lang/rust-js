use dep::{pairs, words};

pub fn main() {
    let mut w = words("one two three");
    w.next();
    let rest = w.clone();
    println!("{:?} {:?}", w.collect::<Vec<_>>(), rest.count());

    let v = [1, 2, 3];
    let p = pairs(&v);
    println!("{} {:?}", p.clone().count(), p.collect::<Vec<_>>());
}
