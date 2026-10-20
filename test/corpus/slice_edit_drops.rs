// A vector's edits of values with a destructor drop what they replace or
// cut off as std's do (ADR 0507): `fill` each item it writes over, by the
// type's own `clone_from` where it has one, `resize` what it cuts off and
// then the value, `clone_from_slice` each it replaces, `dedup` each repeat,
// by `==`; and `extend_from_within`, `first_mut`, `last_mut`,
// `split_at_mut`, `split_first_mut` and `chunks_mut` drop none.
#[derive(Clone, PartialEq, Debug)]
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
#[derive(Debug)]
struct Tracked(u8);
impl Clone for Tracked {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Tracked(self.0)
    }
    fn clone_from(&mut self, source: &Self) {
        println!("clone_from {} over {}", source.0, self.0);
        self.0 = source.0;
    }
}
#[derive(Debug)]
enum Mode {
    On,
    Off,
}
impl Clone for Mode {
    fn clone(&self) -> Self {
        match self {
            Mode::On => Mode::On,
            Mode::Off => Mode::Off,
        }
    }
    fn clone_from(&mut self, source: &Self) {
        println!("mode clone_from {source:?} over {self:?}");
        *self = source.clone();
    }
}
#[derive(PartialEq, Debug)]
struct Point(i32, i32);
fn ids(v: &[Loud]) -> Vec<u8> {
    v.iter().map(|l| l.0).collect()
}
fn fill() {
    let mut v = vec![Loud(1), Loud(2), Loud(3)];
    v.fill(Loud(4));
    println!("filled {:?}", ids(&v));
    let mut none: Vec<Loud> = Vec::new();
    none.fill(Loud(5));
    let mut t = vec![Tracked(1), Tracked(2)];
    t.fill(Tracked(3));
    println!("{t:?}");
    let mut modes = [Mode::Off, Mode::Off, Mode::On];
    modes.fill(Mode::On);
    println!("{modes:?}");
}
fn resize() {
    let mut v = vec![Loud(1), Loud(2), Loud(3)];
    v.resize(1, Loud(6));
    println!("shrunk {:?}", ids(&v));
    v.resize(3, Loud(7));
    println!("grown {:?}", ids(&v));
    v.clone_from_slice(&[Loud(8), Loud(9), Loud(10)]);
    println!("cloned {:?}", ids(&v));
}
fn dedup() {
    let mut d = vec![Loud(11), Loud(11), Loud(12), Loud(12), Loud(11)];
    d.dedup();
    println!("deduped {:?}", ids(&d));
    let mut p = vec![Point(1, 2), Point(1, 2), Point(3, 4)];
    p.dedup();
    println!("{p:?}");
}
fn lend() {
    let mut d = vec![Loud(11), Loud(12), Loud(13)];
    d.extend_from_within(1..);
    println!("extended {:?}", ids(&d));
    if let Some(first) = d.first_mut() {
        first.0 = 20;
    }
    if let Some(last) = d.last_mut() {
        let old = std::mem::replace(last, Loud(21));
        println!("replaced {}", old.0);
    }
    let (a, b) = d.split_at_mut(1);
    std::mem::swap(&mut a[0], &mut b[0]);
    if let Some((head, rest)) = d.split_first_mut() {
        head.0 += rest.len() as u8;
    }
    for chunk in d.chunks_mut(2) {
        chunk.reverse();
    }
    println!("edited {:?}", ids(&d));
}
fn main() {
    fill();
    resize();
    dedup();
    lend();
    println!("end");
}
