// An object made where it's used that reads a reference, `View { label: s,
// n }`, past a drop of what has a destructor: made where it's given, after
// the drop, not settled into a variable before it, as an element is.
struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
#[derive(Debug)]
struct View<'a> {
    label: &'a str,
    n: usize,
}
fn shown(v: View) -> String {
    format!("{v:?}")
}
fn made(s: &str, flag: bool) -> String {
    let _g = Loud(1);
    shown(View {
        label: s,
        n: if flag { s.len() } else { 0 },
    })
}
fn held<'a>(s: &'a str, flag: bool) -> View<'a> {
    let _g = Loud(2);
    let mut n = 0;
    if flag {
        n = s.len();
    }
    View { label: s, n }
}
fn branchy(s: &str, k: u32) -> String {
    let g = Loud(3);
    let v = View {
        label: &s[1..],
        n: k as usize,
    };
    let out = match k {
        0 => {
            drop(g);
            shown(v)
        }
        _ => shown(v),
    };
    out
}
fn main() {
    println!("{}", made("abc", true));
    println!("{:?}", held("de", true));
    println!("{} {}", branchy("xyz", 0), branchy("pq", 2));
}
