// A slice's searches by a predicate that runs the program's code, `split`,
// `splitn`, `rsplit`, `rsplitn`, `split_inclusive`, `chunk_by` and their
// `_mut`s, ask it as std's iterators do: as each piece is stepped, from the
// front or from the back, and of no item past what's stepped, their hints
// std's, `chunk_by`'s `last` its `next_back`, a `_mut`'s pieces written
// through, and one whose predicate panics part-way. Each asked it of every
// item before the first piece (#1501). A predicate of std's,
// `u8::is_ascii_whitespace`, runs none, and stays the array.
fn main() {
    let v = [1, 0, 2, 0, 3];
    for part in v.split(|x| {
        println!("test {x}");
        *x == 0
    }) {
        println!("part {part:?}");
    }
    let mut it = v.split(|x| {
        println!("b {x}");
        *x == 0
    });
    println!("{:?} {:?} {:?}", it.next_back(), it.size_hint(), it.next());
    println!("{:?}", it.collect::<Vec<_>>());
    let first: Vec<&[i32]> = v
        .split(|x| {
            println!("t {x}");
            *x == 0
        })
        .take(1)
        .collect();
    println!("{first:?}");
    let rs: Vec<&[i32]> = v
        .rsplit(|x| {
            println!("rs {x}");
            *x == 0
        })
        .collect();
    println!("{rs:?}");
    let mut sn = v.splitn(2, |x| {
        println!("sn {x}");
        *x == 0
    });
    println!("{:?} {:?}", sn.size_hint(), sn.next());
    println!("{:?} {:?} {:?}", sn.size_hint(), sn.next(), sn.next());
    let rsn: Vec<&[i32]> = v
        .rsplitn(2, |x| {
            println!("rsn {x}");
            *x == 0
        })
        .collect();
    println!("{rsn:?}");
    let mut si = v.split_inclusive(|x| {
        println!("si {x}");
        *x == 0
    });
    println!("{:?} {:?} {:?}", si.next_back(), si.next(), si.size_hint());
    println!("{:?}", si.collect::<Vec<_>>());
    let ends: Vec<&[i32]> = [1, 0]
        .split_inclusive(|x| {
            println!("end {x}");
            *x == 0
        })
        .collect();
    let ends_back: Vec<&[i32]> = [1, 0].split_inclusive(|x| *x == 0).rev().collect();
    println!("{ends:?} {ends_back:?}");
    let e: [i32; 0] = [];
    println!(
        "{:?} {:?}",
        e.split_inclusive(|x| *x == 0).count(),
        e.split(|x| *x == 0).count()
    );
    let w = [1, 1, 2, 3, 3, 3, 1];
    let mut cb = w.chunk_by(|a, b| {
        println!("pair {a} {b}");
        a == b
    });
    println!("{:?} {:?} {:?}", cb.next(), cb.next_back(), cb.size_hint());
    println!("{:?}", cb.collect::<Vec<_>>());
    println!(
        "{}",
        w.chunk_by(|a, b| {
            println!("cnt {a} {b}");
            a == b
        })
        .count()
    );
    println!(
        "{:?}",
        w.chunk_by(|a, b| {
            println!("last {a} {b}");
            a == b
        })
        .last()
    );
    let mut m = [5, 0, 6, 7, 0, 8];
    for piece in m.split_mut(|x| {
        println!("m {x}");
        *x == 0
    }) {
        if let Some(first) = piece.first_mut() {
            *first *= 10;
        }
    }
    println!("{m:?}");
    let mut c = [1, 1, 2, 2, 2];
    for run in c.chunk_by_mut(|a, b| a == b) {
        run[0] = run.len() as i32 * 100;
    }
    println!("{c:?}");
    let r = std::panic::catch_unwind(|| {
        for p in [1, 2, 3, 0, 4].split(|x| {
            if *x == 4 {
                panic!("at 4")
            }
            *x == 0
        }) {
            println!("before panic {p:?}");
        }
    });
    println!("{}", r.is_err());
    let bytes = b"a b";
    println!("{:?}", bytes.split(u8::is_ascii_whitespace).collect::<Vec<_>>());
}
