struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let p = dep::pair(vec![Loud(1), Loud(2)].into_iter());
    println!("{}", p.shown());
    let louds: Vec<u32> = p.map(|l| l.0).collect();
    println!("{louds:?}");
}
