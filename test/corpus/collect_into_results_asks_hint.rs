// A collect into a `Result` or an `Option` of a collection asks its
// iterator's `size_hint` through std's `GenericShunt`, whose hint is its
// source's, its lower bound none, while no `Err` or `None` has stopped it
// (#848): of a `Vec`, a `VecDeque`, a `Box<[T]>`, a heap or a B-tree,
// after its first value, then each time the `Vec` std grows is full; of a
// `String` of `char`s or a hash table, once, before it's stepped; of a
// `LinkedList` or a `String` of strings, never. One stopped by its first
// item asks none.
use std::collections::{BTreeSet, BinaryHeap, HashMap, LinkedList, VecDeque};

struct Loud {
    n: u32,
    fail_at: Option<u32>,
}

impl Iterator for Loud {
    type Item = Result<u32, String>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        match self.fail_at == Some(self.n) {
            true => Some(Err(format!("at {}", self.n))),
            false => Some(Ok(self.n)),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        print!("[hint {}] ", self.n);
        (self.n as usize, Some(self.n as usize))
    }
}

fn loud(n: u32) -> Loud {
    Loud { n, fail_at: None }
}

fn failing(n: u32, at: u32) -> Loud {
    Loud { n, fail_at: Some(at) }
}

fn generic<T: Clone>(items: Vec<T>, n: u32) -> Result<Vec<T>, String> {
    loud(n).map(|r| r.map(|i| items[i as usize % items.len()].clone())).collect()
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        print!("(drop {}) ", self.0);
    }
}

struct Panicky {
    n: u32,
    asked: u32,
}

impl Iterator for Panicky {
    type Item = Option<Noisy>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        Some(Some(Noisy(self.n)))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.asked == 1 {
            panic!("hint panics at {}", self.n);
        }
        (0, None)
    }
}

struct PanicLater {
    inner: Panicky,
}

impl Iterator for PanicLater {
    type Item = Option<Noisy>;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.inner.n < 5 {
            panic!("hint panics at {}", self.inner.n);
        }
        (0, None)
    }
}

fn panicky(n: u32) -> Panicky {
    Panicky { n, asked: 0 }
}

fn main() {
    let caught = std::panic::catch_unwind(|| {
        let mut it = panicky(9);
        it.asked = 1;
        let v: Option<Vec<Noisy>> = it.collect();
        v.map(|v| v.len())
    });
    println!("first ask {}", caught.is_err());
    let caught = std::panic::catch_unwind(|| {
        let v: Option<Vec<Noisy>> = PanicLater { inner: panicky(9) }.collect();
        v.map(|v| v.len())
    });
    println!("later ask {}", caught.is_err());
    let v: Result<Vec<u32>, String> = loud(9).collect();
    println!("vec {:?}", v.map(|v| v.len()));
    let v: Result<Vec<u8>, String> = loud(10).map(|r| r.map(|x| x as u8)).collect();
    println!("bytes {:?}", v.map(|v| v.len()));
    let v: Result<Vec<u32>, String> = failing(9, 3).collect();
    println!("failed {:?}", v);
    let v: Result<Vec<u32>, String> = failing(5, 4).collect();
    println!("failed first {:?}", v);
    let v: Result<VecDeque<u32>, String> = loud(6).collect();
    println!("deque {:?}", v.map(|v| v.len()));
    let v: Result<Box<[u32]>, String> = loud(6).collect();
    println!("box {:?}", v.map(|v| v.len()));
    let v: Result<BinaryHeap<u32>, String> = loud(6).collect();
    println!("heap {:?}", v.map(|v| v.len()));
    let v: Result<BTreeSet<u32>, String> = loud(6).collect();
    println!("btree {:?}", v.map(|v| v.len()));
    let v: Result<LinkedList<u32>, String> = loud(6).collect();
    println!("list {:?}", v.map(|v| v.len()));
    let v: Result<HashMap<u32, u32>, String> = loud(6).map(|r| r.map(|x| (x, x))).collect();
    println!("map {:?}", v.map(|v| v.len()));
    let v: Result<String, String> = loud(6).map(|r| r.map(|x| char::from(b'a' + x as u8))).collect();
    println!("string {:?}", v);
    let v: Result<String, String> = loud(3).map(|r| r.map(|x| x.to_string())).collect();
    println!("strings {:?}", v);
    let v: Option<Vec<u32>> = loud(9).map(|r| r.ok()).collect();
    println!("option {:?}", v.map(|v| v.len()));
    let v: Option<Vec<u32>> = failing(9, 2).map(|r| r.ok()).collect();
    println!("option none {:?}", v);
    println!("generic {:?}", generic(vec!['x', 'y'], 7).map(|v| v.len()));
    println!("generic units {:?}", generic(vec![(), ()], 7).map(|v| v.len()));
}
