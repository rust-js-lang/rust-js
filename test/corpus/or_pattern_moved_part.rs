// An or-pattern whose alternatives move parts no variant tells apart,
// `(Some(t), _) | (_, Some(t))`: `t` is the part that matched, and the
// other is dropped.
pub struct D(u32);
impl Drop for D { fn drop(&mut self) { println!("drop {}", self.0); } }
pub fn f(p: (Option<D>, Option<D>)) -> Option<D> { match p { (Some(t), _) | (_, Some(t)) => Some(t), _ => None } }
fn main() {
    let r = f((Some(D(1)), Some(D(2)))); println!("got {:?}", r.as_ref().map(|d| d.0)); drop(r);
    let r = f((None, Some(D(3)))); println!("got {:?}", r.as_ref().map(|d| d.0)); drop(r);
    let r = f((None, None)); println!("got {:?}", r.as_ref().map(|d| d.0));
}
