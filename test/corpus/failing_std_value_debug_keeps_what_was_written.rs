// A std value's `Debug`, an `Option`'s, a `Result`'s, a tuple's, a
// `Vec`'s, an array's, a slice's, a `VecDeque`'s, a set's or a map's, of a
// part that returns `Err`, first, in the middle or last, written into a
// `String`: the write gives the `Err`, and the `String` keeps what std's
// builders wrote before it, `Some(B`, `[None, Some(B`, as std's `Debug` of
// each is its builder's (#1561). `{:?}` and `{:#?}`; nested; and one whose
// parts can't fail, written as before.
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt::{self, Write};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bad(u8);

impl fmt::Debug for Bad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "B{}\nb", self.0)?;
        Err(fmt::Error)
    }
}

fn written(show: impl Fn(&mut String) -> fmt::Result) {
    let mut s = String::new();
    let r = show(&mut s);
    println!("{r:?} {s:?}");
}

macro_rules! both {
    ($value:expr) => {{
        let value = $value;
        written(|s| write!(s, "{value:?}"));
        written(|s| write!(s, "{value:#?}"));
    }};
}

fn main() {
    both!(Some(Bad(1)));
    both!(None::<Bad>);
    both!(Ok::<Bad, u8>(Bad(2)));
    both!(Err::<u8, Bad>(Bad(3)));
    both!((1, Bad(4), 2));
    both!((Bad(5),));
    both!(vec![Bad(6), Bad(7)]);
    both!(vec![None, Some(Bad(8)), None]);
    both!([Some(1), None]);
    both!(&[Some(Bad(15)), None][..]);
    both!(VecDeque::from([Some(Bad(9))]));
    both!(BTreeSet::from([Bad(10)]));
    both!(BTreeMap::from([(1, Some(Bad(11))), (2, None)]));
    both!(BTreeMap::from([(Bad(12), 1)]));
    both!(HashMap::from([(1, Bad(13))]));
    both!(Some(vec![(1, Bad(14))]));
    // Made where it's shown, a tuple's and an `Option`'s.
    written(|s| write!(s, "{:?}", (1, Bad(16))));
    written(|s| write!(s, "{:?}", Some((Bad(17), 2))));
    let mut s = String::new();
    let r = write!(s, "a {:?} b", vec![Some(1), Some(2)]);
    println!("{r:?} {s:?}");
}
