// A trait's generic method, in a crate where a type has a destructor: what
// it's given is dropped where Rust drops it, called on the impl itself or
// through a dictionary, as a generic function's is (ADR 0098). Its drops
// are decided by the trait's declaration, so both sides agree.

use std::fmt::Debug;

#[derive(Debug)]
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Store {
    fn put<T: Debug>(&mut self, item: T) -> usize;
    fn put_all<I: IntoIterator<Item = String>>(&mut self, items: I);
    fn keep<T: Debug>(&mut self, item: T) -> T {
        self.put(format!("kept {item:?}"));
        item
    }
}

struct Log(Vec<String>);

impl Store for Log {
    fn put<T: Debug>(&mut self, item: T) -> usize {
        self.0.push(format!("{item:?}"));
        println!("put {}", self.0.len());
        self.0.len()
    }

    fn put_all<I: IntoIterator<Item = String>>(&mut self, items: I) {
        for item in items {
            self.0.push(item);
        }
    }
}

fn fill<S: Store>(store: &mut S) {
    store.put(1);
    store.put(Noisy("through a dictionary"));
    println!("after the dictionary's put");
    let back = store.keep(Noisy("kept"));
    println!("got {:?} back", back);
}

fn main() {
    let mut log = Log(Vec::new());
    log.put(Noisy("direct"));
    println!("after the direct put");
    fill(&mut log);
    log.put_all(vec!["a".to_string(), "b".to_string()]);
    println!("{:?}", log.0);
}
