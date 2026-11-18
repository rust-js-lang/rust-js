// `Box<dyn Fn()>`s holding a value with a destructor, kept in a `Vec`
// whose items generic code and std call by `&` alone, then dropped: no call
// by value reaches them, so they're given as they are and dropped with the
// `Vec` (#2050's leave-alone).
struct Loud(&'static str);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn each<F: Fn() -> u32>(fs: &[F]) -> u32 {
    fs.iter().map(|f| f() + f()).sum()
}

fn main() {
    let d = Loud("by ref");
    let b: Box<dyn Fn() -> u32> = Box::new(move || {
        let held = &d;
        println!("in {}", held.0);
        3
    });
    let fs = vec![b];
    println!("{}", each(&fs));
    println!("{}", fs.iter().map(|f| f()).sum::<u32>());
    drop(fs);
    println!("end");
}
