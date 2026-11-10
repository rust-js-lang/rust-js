use dep::{cloned, dup};

#[derive(Clone, Copy, Debug)]
struct P {
    x: u32,
}

fn each<F: Fn(P) -> (P, P)>(f: F) -> (P, P) {
    f(P { x: 3 })
}

pub fn main() {
    let (mut a, b) = dup(P { x: 1 });
    a.x = 9;
    println!("{a:?} {b:?}");
    let p: fn(P) -> (P, P) = dup::<P>;
    let (mut c, d) = p(P { x: 2 });
    c.x = 9;
    println!("{c:?} {d:?}");
    let d2: &dyn Fn(P) -> (P, P) = &dup::<P>;
    let (mut e, f) = d2(P { x: 4 });
    e.x = 9;
    println!("{e:?} {f:?}");
    let (mut g, h) = each(dup);
    g.x = 9;
    println!("{g:?} {h:?}");
    let v = vec![1u32];
    let (mut i, j) = cloned(&v);
    i.push(2);
    println!("{i:?} {j:?} {v:?}");
    let q: fn(&Vec<u32>) -> (Vec<u32>, Vec<u32>) = cloned::<Vec<u32>>;
    let (mut k, l) = q(&v);
    k.push(3);
    println!("{k:?} {l:?}");
    let r: &dyn Fn(&Vec<u32>) -> (Vec<u32>, Vec<u32>) = &cloned::<Vec<u32>>;
    let (mut m, n) = r(&v);
    m.push(4);
    println!("{m:?} {n:?}");
}
