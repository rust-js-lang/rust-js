// A `str`'s searches by a pattern that runs the program's code, a closure
// or a function of the crate's, ask it as std's iterators do: as each piece
// is stepped, from the front or from the back, and of no `char` past what's
// stepped. `split`, `splitn`, `rsplit`, `rsplitn`, `split_terminator`,
// `rsplit_terminator`, `split_inclusive`, `matches`, `rmatches`,
// `match_indices` and `rmatch_indices`, stepped from each end and stopped
// early, their hint std's `(0, None)`, and one whose closure panics part-way.
// Each asked it of every `char` before the first piece (#1501). A pattern
// of std's, `char::is_whitespace`, runs none, and stays the array. And what's
// made of one stepped from its back, a map, reversed or stepped so.
fn first() {
    for part in "ab,c".split(|c: char| {
        println!("test {c}");
        c == ','
    }) {
        println!("part {part}");
    }
    let mut it = "x;y;z".split(|c: char| {
        println!("t2 {c}");
        c == ';'
    });
    println!("{:?}", it.next());
    println!("after first");
    let n: Vec<&str> = "a b"
        .splitn(2, |c: char| {
            println!("t3 {c}");
            c == ' '
        })
        .collect();
    println!("{n:?}");
    for p in "1-2-3".rsplit(|c: char| {
        println!("t4 {c}");
        c == '-'
    }) {
        println!("r {p}");
    }
    for p in "q.w.".split_terminator(|c: char| {
        println!("t5 {c}");
        c == '.'
    }) {
        println!("st {p}");
    }
    for p in "k1m2".split_inclusive(|c: char| {
        println!("t6 {c}");
        c.is_ascii_digit()
    }) {
        println!("si {p}");
    }
}

/// What's made of one stepped from its back: a map of it, reversed or
/// asked its `next_back()`, and a `rev()` stopped after one piece.
fn backs() {
    let v: Vec<String> = "a,b,c"
        .split(|c: char| {
            println!("m {c}");
            c == ','
        })
        .map(|p| p.to_uppercase())
        .rev()
        .collect();
    println!("{v:?}");
    let mut m = "x;y;z".split(|c: char| c == ';').map(|p| p.len());
    println!("{:?} {:?}", m.next_back(), m.next());
    let r: Vec<&str> = "1|2|3"
        .split(|c: char| {
            println!("r {c}");
            c == '|'
        })
        .rev()
        .take(1)
        .collect();
    println!("{r:?}");
}

fn main() {
    first();
    backs();
    let m: Vec<(usize, &str)> = "a1b2"
        .match_indices(|c: char| {
            println!("mi {c}");
            c.is_ascii_digit()
        })
        .take(1)
        .collect();
    println!("{m:?}");
    let rm: Vec<(usize, &str)> = "né1b2"
        .rmatch_indices(|c: char| {
            println!("rmi {c}");
            c.is_ascii_digit()
        })
        .collect();
    println!("{rm:?}");
    let mt: Vec<&str> = "a1b2"
        .matches(|c: char| {
            println!("m {c}");
            c.is_ascii_digit()
        })
        .take(1)
        .collect();
    println!("{mt:?}");
    let rmt: Vec<&str> = "a1b2"
        .rmatches(|c: char| {
            println!("rm {c}");
            c.is_ascii_digit()
        })
        .collect();
    println!("{rmt:?}");
    let mut sp = "a,b,,c".split(|c: char| {
        println!("s {c}");
        c == ','
    });
    println!("{:?} {:?}", sp.next_back(), sp.next());
    println!("{:?}", sp.collect::<Vec<_>>());
    let mut st = "a.b.".split_terminator(|c: char| {
        println!("st {c}");
        c == '.'
    });
    println!("{:?} {:?} {:?}", st.next_back(), st.next_back(), st.next_back());
    let rst: Vec<&str> = "a.b.".rsplit_terminator(|c: char| c == '.').collect();
    println!("{rst:?}");
    let mut si = "x1y2".split_inclusive(|c: char| {
        println!("si {c}");
        c.is_ascii_digit()
    });
    println!("{:?} {:?} {:?}", si.next_back(), si.next(), si.next());
    let rn: Vec<&str> = "1-2-3-4"
        .rsplitn(2, |c: char| {
            println!("rn {c}");
            c == '-'
        })
        .collect();
    println!("{rn:?}");
    let sn: Vec<&str> = "1-2-3-4".splitn(3, |c: char| c == '-').collect();
    println!("{sn:?} {:?}", "".split(|c: char| c == 'x').collect::<Vec<_>>());
    fn is_comma(c: char) -> bool {
        println!("fn {c}");
        c == ','
    }
    for p in "p,q".split(is_comma) {
        println!("p {p}");
    }
    let hint = "a,b".split(|c: char| c == ',').size_hint();
    println!("{hint:?} {:?}", "a,b".split(char::is_whitespace).collect::<Vec<_>>());
    let r = std::panic::catch_unwind(|| {
        for p in "1,2,3".split(|c: char| {
            if c == '3' {
                panic!("at 3")
            }
            c == ','
        }) {
            println!("before panic {p}");
        }
    });
    println!("{}", r.is_err());
}
