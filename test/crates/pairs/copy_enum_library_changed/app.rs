//! A copy of it is its own though only the library changes it in place:
//! what the crate kept stays as it was when the library grows the holder's
//! (ADR 0100).
pub fn main() {
    let mut holder = dep::held();
    let kept = holder.shape;
    dep::grow(&mut holder);
    println!("{:?} {kept:?}", holder.shape);
}
