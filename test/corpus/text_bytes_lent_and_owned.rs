// A text's bytes lent, `as_bytes()`, `bytes()` and `AsRef<[u8]>`, and owned,
// `into_bytes()`, `into_boxed_bytes()`, `Vec::from`, each way they're used,
// of a short text and a long one, whose bytes JS keeps (ADR 0126, amended,
// #1184): what's owned is changed, and the text's bytes never are.
fn first<B: AsRef<[u8]>>(b: B) -> u8 {
    b.as_ref()[0]
}

fn uses(s: &str) {
    let mut a = s.as_bytes().to_vec();
    a[0] = 1;
    let mut b = s.as_bytes().to_owned();
    b[1] = 2;
    let mut c = Vec::from(s.as_bytes());
    c[2] = 3;
    let mut d: Vec<u8> = s.as_bytes().into();
    d[3] = 4;
    let mut e: Vec<u8> = s.as_bytes().iter().copied().collect();
    e[4] = 5;
    let mut f: [u8; 6] = s.as_bytes()[..6].try_into().unwrap();
    f[5] = 6;
    let mut g = s.to_string().into_bytes();
    g[0] = 7;
    let mut i = s.bytes().collect::<Vec<u8>>();
    i.sort();
    i.reverse();
    let mut j = s.as_bytes().repeat(2);
    j[0] = 10;
    let mut k = [s.as_bytes(), b"!"].concat();
    k[0] = 11;
    let mut l = s.as_bytes().to_ascii_uppercase();
    l[0] = 12;
    let m: Vec<u8> = s.bytes().rev().collect();
    let mut n = Vec::new();
    n.extend_from_slice(s.as_bytes());
    n[0] = 13;
    let mut o: Box<[u8]> = s.to_string().into_boxed_str().into_boxed_bytes();
    o[0] = 14;
    let mut q: Vec<u8> = s.into();
    q[0] = 15;
    let r: Vec<u8> = s.as_bytes().iter().map(|x| x + 1).collect();
    let mut t = s.as_bytes().to_vec();
    t.sort_unstable();
    t.dedup();
    let total: u32 = s.bytes().map(u32::from).sum();
    let p = s.as_bytes().as_ptr();
    let second = unsafe { *p.add(1) };
    let third = unsafe { s.as_ptr().add(2).read() };
    let chunks: Vec<&[u8]> = s.as_bytes().chunks(4).take(2).collect();
    let (left, right) = s.as_bytes().split_at(3);
    println!("{a:?}\n{b:?}\n{c:?}\n{d:?}\n{e:?}\n{f:?}\n{g:?}\n{i:?}\n{j:?}\n{k:?}\n{l:?}\n{m:?}\n{n:?}\n{o:?}\n{q:?}\n{r:?}\n{t:?}");
    println!("{total} {second} {third} {chunks:?} {left:?} {} {}", right.len(), first(s));
    println!("{:?} {} {}", s.as_bytes().first(), s.as_bytes().len(), s.as_bytes().iter().rposition(|&x| x == b'l').unwrap_or(99));
    println!("{s}");
}

fn main() {
    uses("héllo, wörld");
    let long = "héllo, wörld, ".repeat(10);
    uses(&long);
    uses(&long);
    let ascii = "hello, world, ".repeat(10);
    uses(&ascii);
}
