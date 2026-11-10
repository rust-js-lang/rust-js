use dep::{counter, lengths, numbered, running, scaled};

pub fn main() {
    let words = vec!["one".to_string(), "three".to_string(), "five".to_string()];
    let l = lengths(words);
    println!("{} {:?}", l.clone().sum::<usize>(), l.collect::<Vec<_>>());

    let mut r = running(vec![1, 2, 3, 4]);
    r.next();
    let rest = r.clone();
    println!("{:?} {:?}", r.collect::<Vec<_>>(), rest.collect::<Vec<_>>());

    let mut c = counter();
    c();
    let mut d = c.clone();
    println!("{} {} {}", c(), d(), c());

    let v = [1, 2, 3];
    let s = scaled(&v, 10);
    println!("{:?} {}", s.clone().collect::<Vec<_>>(), s.max().unwrap_or(0));

    let mut n = numbered(vec![5, 6, 7]);
    n.next();
    let again = n.clone();
    println!("{:?} {:?}", n.collect::<Vec<_>>(), again.collect::<Vec<_>>());
}
