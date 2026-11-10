// A `Vec` collected or extended, in a crate that reads a capacity, of a
// string's pattern's iterator, `split(..)`, `lines()`, `matches(..)` and the
// rest, whose `size_hint` is std's default, no lower bound, where JS counts
// the array of what it gives: given to generic code that grows a `Vec` of
// it, and collected where it's made, which was refused (#1180's (3)).
fn chained<I: Iterator<Item = char>>(it: I) -> usize {
    let v: Vec<char> = it.chain(std::iter::once('x')).collect();
    v.capacity()
}

fn words<'a, I: Iterator<Item = &'a str>>(it: I) -> usize {
    let v: Vec<&str> = it.chain(std::iter::once("x")).collect();
    v.capacity()
}

fn extended<I: Iterator<Item = char>>(it: I) -> usize {
    let mut v: Vec<char> = Vec::new();
    v.extend(it);
    v.capacity()
}

fn extended_words<'a, I: Iterator<Item = &'a str>>(it: I) -> usize {
    let mut v: Vec<&str> = Vec::new();
    v.extend(it);
    v.capacity()
}

fn main() {
    let s = "héllo wörld, this is a longer text, with commas, and more words in it";
    println!("{}", chained(s.chars()));
    println!("{}", words(s.split(',')));
    println!("{}", extended(s.chars()));
    println!("{}", extended_words(s.split(',')));
    println!("{}", words(s.split_whitespace()));
    println!("{}", extended(s.chars().rev()));
    direct();
}

fn direct() {
    let s = "héllo wörld, this is a longer text, with commas, and more words in it\nsecond line\nthird";
    let a: Vec<&str> = s.split(',').collect();
    let b: Vec<&str> = s.split_whitespace().collect();
    let c: Vec<&str> = s.lines().collect();
    let d: Vec<&str> = s.matches("i").collect();
    let e: Vec<(usize, &str)> = s.match_indices(", ").collect();
    let f: Vec<&str> = s.splitn(3, ' ').collect();
    let g: Vec<&str> = s.rsplit(' ').collect();
    let h: Vec<&str> = s.split_terminator(',').collect();
    let i: Vec<&str> = s.split_inclusive(' ').skip(1).collect();
    let j: Vec<&str> = s.split(',').rev().collect();
    let k: Vec<usize> = s.split(' ').map(|w| w.len()).collect();
    let l: Vec<&str> = s.split_ascii_whitespace().collect();
    println!(
        "{} {} {} {} {} {} {} {} {} {} {} {}",
        a.capacity(),
        b.capacity(),
        c.capacity(),
        d.capacity(),
        e.capacity(),
        f.capacity(),
        g.capacity(),
        h.capacity(),
        i.capacity(),
        j.capacity(),
        k.capacity(),
        l.capacity()
    );
}
