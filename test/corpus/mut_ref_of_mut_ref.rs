// A `&mut` to a variable that's a `&mut` itself, of a number's or a
// string's cell too: that variable, where it's used, a closure's too.
fn main() {
    let mut x = String::from("a");
    let mut mref_x = &mut x;
    let m_mref_x = &mut mref_x;
    let mut c = || {
        **m_mref_x = String::from("b");
        m_mref_x.push('c');
    };
    c();
    let mut n = 1;
    let mut r = &mut n;
    let rr = &mut r;
    **rr += 5;
    let mut other = 100;
    *rr = &mut other;
    **rr += 1;
    println!("{} {} {}", x, n, other);
}
