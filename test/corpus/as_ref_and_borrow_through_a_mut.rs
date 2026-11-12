// `s.as_ref()` and `r.borrow()` of a `&mut` to text or a number are std's
// forwarding impls, `AsRef<U> for &mut T` and `Borrow<T> for &mut T`, their
// receiver a `&&mut T`: the `&` they give is to what's in the `&mut`'s cell,
// its `value`, where they gave the cell itself, `[object Object]` shown and
// a length of 0 (#1486). A `&mut` to a `Vec`, the array itself, is as it was.
use std::borrow::Borrow;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Level {
    Low,
    High(u8),
}

struct Named<'a> {
    name: &'a mut String,
    count: &'a mut u32,
}

impl Named<'_> {
    fn shown(&self) -> String {
        let name: &str = self.name.as_ref();
        let count: &u32 = self.count.borrow();
        format!("{name}:{count}")
    }

    fn renamed(&mut self, to: &str) -> usize {
        self.name.push_str(to);
        let name: &str = self.name.as_ref();
        name.len()
    }
}

fn len(s: &mut String) -> usize {
    let b: &str = s.as_ref();
    b.len()
}

fn upper(s: &mut String) -> String {
    let b: &str = s.as_ref();
    let shout = b.to_uppercase();
    s.push('!');
    shout
}

fn doubled(n: &mut i32) -> i32 {
    let b: &i32 = n.borrow();
    let twice = *b * 2;
    *n += 1;
    twice
}

fn whole(s: &mut String) -> usize {
    let b: &String = s.borrow();
    b.len() + b.matches('b').count()
}

fn level(l: &mut Level) -> Level {
    let b: &Level = l.borrow();
    *b
}

fn items(v: &mut Vec<i32>) -> usize {
    let b: &[i32] = v.as_ref();
    let c: &Vec<i32> = v.borrow();
    b.len() + c.len()
}

fn generic<T: std::fmt::Debug>(tag: T, s: &mut String, n: &mut f64) -> String {
    let b: &str = s.as_ref();
    let m: &f64 = n.borrow();
    format!("{tag:?} {b} {m}")
}

// Generic code given a `&mut` as its `K: Borrow<i32>`: the forwarding
// impl's dictionary, whose `borrow` reads the cell `K` is.
fn key<K: Borrow<i32>>(k: K) -> i32 {
    *k.borrow() + 1
}

fn shout<K: Borrow<String>>(k: &K) -> String {
    let s: &String = k.borrow();
    s.to_uppercase()
}

fn main() {
    let mut s = String::from("xy");
    println!("{} {}", len(&mut s), upper(&mut s));
    println!("{s}");

    let mut t = String::from("héllo");
    let r = &mut t;
    let b: &str = r.as_ref();
    println!("{b} {}", b.len());
    let rr = &r;
    let c: &str = rr.as_ref();
    println!("{c}");
    r.push('s');
    let d: &String = r.borrow();
    println!("{d}");

    let mut n = 5;
    println!("{} {n}", doubled(&mut n));
    let r = &mut n;
    let b: &i32 = r.borrow();
    println!("{b} {}", b + 1);
    *r += 10;
    let b: &i32 = r.borrow();
    println!("{b}");

    // What gives a `&mut` as it's given keeps its cell: a `&` to the `&mut`
    // itself, and a `Box` of it, each written through after.
    let again: &&mut i32 = r.borrow();
    println!("{}", **again + 1);
    let same = std::convert::identity(&r);
    println!("{}", **same + 2);
    let mut boxed = Box::new(&mut *r);
    **boxed += 1;
    println!("{boxed}");
    println!("{n}");

    println!("{} {} {}", key(&mut n), key(&n), key(n));
    let mut quiet = String::from("quiet");
    let r = &mut quiet;
    println!("{}", shout(&r));

    let mut flag = true;
    let r = &mut flag;
    let b: &bool = r.borrow();
    println!("{b} {}", !*b);
    let mut letter = 'a';
    let r = &mut letter;
    let b: &char = r.borrow();
    println!("{b} {}", b.is_alphabetic());

    println!("{}", whole(&mut String::from("abc")));
    let mut l = Level::High(3);
    println!("{:?} {:?}", level(&mut l), level(&mut Level::Low));
    let mut v = vec![1, 2, 3];
    println!("{}", items(&mut v));

    let mut name = String::from("n");
    let mut count = 2;
    let mut named = Named { name: &mut name, count: &mut count };
    println!("{} {}", named.shown(), named.renamed("ame"));
    println!("{}", named.shown());

    let texts = |s: &mut String| -> usize {
        let b: &str = s.as_ref();
        b.chars().count()
    };
    println!("{}", texts(&mut String::from("été")));

    let mut f = 1.5;
    println!("{}", generic((1, 'a'), &mut String::from("g"), &mut f));

    // Each written out, and given as a function where a closure would be.
    let mut words = vec![String::from("a"), String::from("bc")];
    let lens: Vec<usize> = words.iter_mut().map(|w| AsRef::<str>::as_ref(&w).len()).collect();
    println!("{lens:?}");
    let held: Vec<&mut String> = words.iter_mut().collect();
    let texts: Vec<&str> = held.iter().map(AsRef::as_ref).collect();
    println!("{texts:?}");
    let mut numbers = [1, 2, 3];
    let total: i32 = numbers.iter_mut().map(|n| *Borrow::<i32>::borrow(&n)).sum();
    println!("{total}");
    let held: Vec<&mut i32> = numbers.iter_mut().collect();
    let total: i32 = held.iter().map(Borrow::<i32>::borrow).sum();
    println!("{total}");
}
