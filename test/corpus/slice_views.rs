// A `&mut` of part of a slice is a view of those items: what's written
// through it, by an index, `sort`, `reverse`, `fill`, `swap` or another
// view of it, is written to the slice itself.

fn bump_all(part: &mut [i32]) {
    for x in part.iter_mut() {
        *x += 100;
    }
}

fn tail(v: &mut Vec<i32>) -> &mut [i32] {
    &mut v[2..]
}

#[derive(Debug)]
struct Point {
    x: i32,
}

fn main() {
    let mut v = vec![5, 3, 9, 1, 4, 8];
    v[1..4].sort();
    println!("{v:?}");
    v[..3].reverse();
    println!("{v:?}");
    v[4..].fill(0);
    println!("{v:?}");
    let part = &mut v[1..3];
    part[0] = 7;
    part.swap(0, 1);
    println!("{} {v:?}", part.len());
    bump_all(&mut v[..2]);
    println!("{v:?}");
    tail(&mut v)[0] = -1;
    println!("{v:?}");
    v[1..5][1..3].rotate_left(1);
    println!("{v:?}");
    let all = &mut v[..];
    all[5] = 6;
    println!("{v:?}");

    let (left, right) = v.split_at_mut(3);
    left[0] = right[0];
    right[2] = left.len() as i32;
    println!("{v:?}");
    println!("{:?}", v.split_at_mut_checked(7).is_none());
    unsafe {
        let (a, b) = v.split_at_unchecked(4);
        println!("{a:?} {b:?}");
        let (_, b) = v.split_at_mut_unchecked(5);
        b[0] += 1;
    }
    if let Some((a, b)) = v.split_at_mut_checked(6) {
        a[0] = b.len() as i32;
    }
    println!("{v:?}");

    if let Some((first, rest)) = v.split_first_mut() {
        *first = 10;
        rest[0] = 11;
    }
    if let Some((last, rest)) = v.split_last_mut() {
        *last = rest.len() as i32;
        rest[0] = 20;
    }
    println!("{v:?}");

    for chunk in v.chunks_mut(4) {
        chunk[0] *= 2;
    }
    println!("{v:?}");
    let mut exact = v.chunks_exact_mut(4);
    for chunk in &mut exact {
        chunk.reverse();
    }
    exact.into_remainder()[0] = 0;
    println!("{v:?}");
    for chunk in v.rchunks_mut(4) {
        chunk[0] = -chunk[0];
    }
    for chunk in v.rchunks_exact_mut(5) {
        chunk[4] = 99;
    }
    println!("{v:?}");

    let mut words = vec![1, 0, 2, 3, 0, 4];
    for part in words.split_mut(|x| *x == 0) {
        part.reverse();
        if let Some(first) = part.first_mut() {
            *first += 10;
        }
    }
    println!("{words:?}");
    for part in words.splitn_mut(2, |x| *x == 0) {
        part[0] = -part[0];
    }
    println!("{words:?}");
    for part in words.split_inclusive_mut(|x| *x == 0) {
        part.sort();
    }
    println!("{words:?}");
    for part in words.rsplit_mut(|x| *x == 0) {
        part[0] = 1;
    }
    for part in words.rsplitn_mut(2, |x| *x == 0) {
        part.fill(2);
    }
    println!("{words:?}");

    if let Some(part) = words.get_mut(1..3) {
        part.fill(5);
    }
    println!("{:?} {:?}", words.get_mut(5..9).is_none(), words);
    unsafe {
        words.get_unchecked_mut(..2).fill(6);
        *words.get_unchecked_mut(2) += 1;
    }
    println!("{words:?}");

    if let Some(head) = words.first_chunk_mut::<2>() {
        head[1] = 0;
    }
    if let Some(end) = words.last_chunk_mut::<2>() {
        end[0] = 0;
    }
    println!("{words:?}");
    if let Some((head, rest)) = words.split_first_chunk_mut::<2>() {
        head[0] = rest.len() as i32;
    }
    if let Some((rest, end)) = words.split_last_chunk_mut::<2>() {
        end[1] = rest.len() as i32;
    }
    println!("{words:?}");

    let mut runs = vec![1, 1, 2, 3, 3, 3];
    for run in runs.chunk_by_mut(|a, b| a == b) {
        let n = run.len() as i32;
        run[0] = n;
    }
    println!("{runs:?}");
    println!("{:?}", runs.chunk_by(|a, b| a <= b).collect::<Vec<_>>());

    if let Some(head) = runs.first_chunk_mut::<2>() {
        *head = [7, 8];
    }
    println!("{runs:?}");

    let mut points = vec![Point { x: 1 }, Point { x: 2 }, Point { x: 3 }];
    let (head, tail) = points.split_at_mut(1);
    head[0].x = tail[1].x;
    tail[0] = Point { x: 0 };
    points[1..].swap(0, 1);
    println!("{points:?}");
}
