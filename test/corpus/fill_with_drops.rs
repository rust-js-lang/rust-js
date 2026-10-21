// `fill_with` of items with a destructor: each new one made, then the one it
// replaces dropped, as `*item = f()` does (ADR 0507).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut items = vec![Loud(1), Loud(2), Loud(3)];
    let mut made = 10;
    items.fill_with(|| {
        made += 1;
        println!("make {made}");
        Loud(made)
    });
    println!("{}", items.iter().map(|l| l.0.to_string()).collect::<Vec<_>>().join(" "));
    items[..2].fill_with(|| Loud(0));
    println!("end");
}
