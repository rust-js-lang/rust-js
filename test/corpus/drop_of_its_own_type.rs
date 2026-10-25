// A `Drop` whose body drops a value of its own type calls itself (#229):
// it isn't written at each of its calls, which would make another call to
// write and never end, but called, as Rust's drop glue runs it again.
struct Once(bool);

impl Drop for Once {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if std::mem::replace(&mut self.0, false) {
            let old = std::mem::replace(self, Once(false));
            println!("replaced {}", old.0);
        }
    }
}

fn main() {
    let _a = Once(true);
    let _b = Once(false);
    println!("end");
}
