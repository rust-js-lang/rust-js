// A `match` whose arms move a part of what it matches, `(front, _) if .. =>`,
// or all of it, `both =>`, of values with a destructor: each part drops
// once, in Rust's order, where the arm that leaves it is left or where its
// owner goes, after what follows the `match`. A guard binds by reference
// and moves nothing where it fails; a panic in an arm drops the part it
// moved, then what's left. In generic code, a blanket impl's copied
// default and a `Wrapper<I>` impl's, where it was refused (#1337).
use std::fmt::Display;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Display for Loud {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

struct Trio<T> {
    a: T,
    b: T,
    c: T,
}

fn consume<T: Display>(l: T) {
    println!("consume {l}");
}

fn pair<T: Display>(front: T, back: T, which: u8) {
    println!("pair {which}");
    let pair = (front, back);
    let n = match pair {
        (front, _)
            if {
                println!("guard {front}");
                which == 0
            } =>
        {
            consume(front);
            1
        }
        (_, back) if which == 1 => {
            println!("back arm {back}");
            2
        }
        both => {
            println!("both {} {}", both.0, both.1);
            3
        }
    };
    println!("after match {n}");
}

fn trio<T: Display>(t: Trio<T>, which: u8) {
    println!("trio {which}");
    let n = match t {
        Trio { a, c, .. } if which == 0 => {
            consume(c);
            println!("kept {a}");
            1
        }
        Trio { b, .. } if which == 1 => {
            drop(b);
            2
        }
        whole => {
            let Trio { a, .. } = whole;
            println!("whole kept {a}");
            3
        }
    };
    println!("after match {n}");
}

fn triple<T: Display>(x: (T, T, T), which: u8) {
    println!("triple {which}");
    match x {
        (first, _, last) if which == 0 => {
            println!("ends {first} {last}");
        }
        all => {
            let moved = all;
            println!("all {}", moved.1);
        }
    }
    println!("end of triple");
}

trait Halves: Iterator {
    fn halves(mut self, whole: bool) -> usize
    where
        Self: Sized,
    {
        let front: Vec<Self::Item> = self.by_ref().take(2).collect();
        let back: Vec<Self::Item> = self.collect();
        let pair = (front, back);
        let n = match pair {
            (front, _)
                if {
                    println!("guard {}", front.len());
                    !whole
                } =>
            {
                println!("front only");
                front.len()
            }
            both => {
                println!("both");
                both.0.len() + both.1.len()
            }
        };
        println!("after halves {n}");
        n
    }
}

impl<I: Iterator> Halves for I {}

struct Wrapper<I>(I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

trait Ext: Iterator {
    fn ends(mut self, whole: bool) -> usize
    where
        Self: Sized,
    {
        let front: Vec<Self::Item> = self.by_ref().take(1).collect();
        let back: Vec<Self::Item> = self.collect();
        let pair = (front, back);
        let n = match pair {
            (_, back) if !whole => back.len(),
            both => both.0.len() * 10 + both.1.len(),
        };
        println!("after ends {n}");
        n
    }
}

impl<I: Iterator> Ext for Wrapper<I> {}

trait Boom: Iterator {
    fn boom(mut self, at: usize) -> usize
    where
        Self: Sized,
    {
        let front: Vec<Self::Item> = self.by_ref().take(1).collect();
        let back: Vec<Self::Item> = self.collect();
        let pair = (front, back);
        match pair {
            (_, back) if back.len() > at => {
                println!("back arm {}", back.len());
                assert!(back.len() < at * 2, "boom at {at}");
                back.len()
            }
            both => both.0.len() + both.1.len(),
        }
    }
}

impl<I: Iterator> Boom for Wrapper<I> {}

fn main() {
    for which in 0..3 {
        pair(Loud("front"), Loud("back"), which);
    }
    for which in 0..3 {
        trio(
            Trio {
                a: Loud("a"),
                b: Loud("b"),
                c: Loud("c"),
            },
            which,
        );
    }
    for which in 0..2 {
        triple((Loud("x0"), Loud("x1"), Loud("x2")), which);
    }
    let loud = ["1", "2", "3", "4"];
    println!("{}", loud.into_iter().map(Loud).halves(false));
    println!("{}", loud.into_iter().map(Loud).halves(true));
    println!("{}", Wrapper(loud.into_iter().map(Loud)).ends(false));
    println!("{}", Wrapper(loud.into_iter().map(Loud)).ends(true));
    let r = std::panic::catch_unwind(|| Wrapper(loud.into_iter().map(Loud)).boom(1));
    println!("boom panicked: {}", r.is_err());
    println!("{}", Wrapper(loud.into_iter().map(Loud)).boom(2));
}
