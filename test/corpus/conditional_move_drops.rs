// A value moved on one path only is dropped where its scope ends on the
// other (ADR 0098): a flag says whether it's still there.
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn take(n: Noisy) {
    println!("took {}", n.0);
}

fn main() {
    for i in 0..2 {
        let give = i == 0;
        let n = Noisy(if give { "given" } else { "kept" });
        if give {
            take(n);
        }
        println!("end of {give}");
    }
}
