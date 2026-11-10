//@ library-differs
// A reference to a type parameter whose `IntoIterator` is a bound, `&'a I:
// IntoIterator`, given to generic code: the caller's own reference, which
// no caller makes a JS iterator of, so each std consumer that takes it
// steps what its dictionary's `into_iter` makes, `zip`'s, `eq`'s, `cmp`'s,
// `extend`'s and the rest as `chain`'s and `for`'s; each had zipped,
// compared or pushed the value itself (strsim's `jaro`, #1264).
use std::collections::{BTreeSet, VecDeque};

struct Word<'a>(&'a str);

impl<'a, 'b> IntoIterator for &'a Word<'b> {
    type Item = char;
    type IntoIter = std::str::Chars<'b>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.chars()
    }
}

fn all<'a, I>(word: &'a I) -> Vec<(u8, char)>
where
    &'a I: IntoIterator<Item = char>,
{
    let z: Vec<(u8, char)> = [1, 2, 3].into_iter().zip(word).collect();
    let c: String = "x".chars().chain(word).collect();
    println!("{c}");
    println!("{}", "abc".chars().eq(word));
    println!("{:?}", "abd".chars().cmp(word));
    println!("{}", "abc".chars().ne(word));
    println!("{:?}", "ab".chars().partial_cmp(word));
    println!("{}", "abd".chars().lt(word));
    let mut v = vec!['z'];
    v.extend(word);
    println!("{v:?}");
    let zz: Vec<((u8, char), char)> = [1u8, 2].into_iter().zip(word).zip(word).collect();
    println!("{zz:?}");
    z
}

fn more<'a, I>(word: &'a I)
where
    &'a I: IntoIterator<Item = char>,
{
    let z: Vec<(u8, char)> = std::iter::zip([1u8, 2], word).collect();
    println!("{z:?}");
    let s: String = word.into_iter().collect();
    println!("{s}");
    let mut w = vec!['x', 'y', 'z'];
    let out: Vec<char> = w.splice(1..2, word).collect();
    println!("{w:?} {out:?}");
    let mut d = VecDeque::new();
    d.extend(word);
    println!("{d:?}");
    let mut b = BTreeSet::new();
    b.extend(word);
    println!("{b:?}");
    let mut t = String::from(">");
    t.extend(word);
    println!("{t}");
    for c in word {
        print!("{c}");
    }
    println!();
}

fn main() {
    println!("{:?}", all(&Word("abc")));
    more(&Word("abc"));
}
