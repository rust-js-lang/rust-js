// A `&mut` of a number or a string a field or an item of an object reached
// another way is, `inc(&mut es[0].n)`: given, that object named where it's
// borrowed and the place copied back; returned, `$handleOn` of it.
#[derive(Debug)]
struct E { n: i32, tag: String }
fn inc(x: &mut i32) { *x += 1; }
fn twice(a: &mut i32, b: &mut u8) { *a *= 2; *b += 1; }
fn push_tag(s: &mut String) { s.push('!'); }
fn main() {
    let mut es = vec![E { n: 1, tag: "a".into() }, E { n: 5, tag: "b".into() }];
    inc(&mut es[0].n);
    let i = es.len() - 1;
    inc(&mut es[i].n);
    let mut c = 0u8;
    twice(&mut es[1].n, &mut c);
    push_tag(&mut es[0].tag);
    let mut grid = vec![vec![0i32; 3]; 2];
    inc(&mut grid[1][2]);
    let mut pairs = [(1, 2), (3, 4)];
    inc(&mut pairs[1].0);
    for k in 0..2 {
        inc(&mut es[k].n);
    }
    println!("{es:?} {c} {grid:?} {pairs:?}");
    returned();
}

fn first(v: &mut Vec<(i32, i32)>) -> &mut i32 { &mut v[0].0 }
struct P {
    x: i32,
}
fn x_of(ps: &mut [P], i: usize) -> &mut i32 { &mut ps[i].x }
fn first_x(ps: &mut Vec<P>) -> &mut i32 {
    &mut ps[0].x
}

fn returned() {
    let mut v = vec![(1, 2)];
    *first(&mut v) += 5;
    let r = first(&mut v);
    *r *= 2;
    println!("{v:?}");
    let mut ps = vec![P { x: 1 }, P { x: 2 }];
    *x_of(&mut ps, 1) += 40;
    let h = x_of(&mut ps, 0);
    *h = -*h;
    *first_x(&mut ps) += 100;
    println!("{} {}", ps[0].x, ps[1].x);
}
