// A vector of values with destructors: what moves its items, in it, out or
// in, drops none, and what it takes out it gives back, which its holder
// drops where Rust's does; what drops what it takes out drops each as std
// does, `retain` and `dedup_by_key` before the next is asked, `truncate`
// and `clear` once it's out, in order, a `VecDeque`'s too (ADR 0098).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ids(v: &[Noisy]) -> Vec<u32> {
    v.iter().map(|n| n.0).collect()
}

fn main() {
    let mut v = vec![Noisy(3), Noisy(1), Noisy(2)];
    v.insert(1, Noisy(4));
    println!("{:?}", ids(&v));
    let removed = v.remove(0);
    println!("removed {} {:?}", removed.0, ids(&v));
    drop(removed);
    let swapped = v.swap_remove(0);
    println!("swapped {} {:?}", swapped.0, ids(&v));
    v.push(Noisy(7));
    v.push(Noisy(5));
    v.reverse();
    println!("{:?}", ids(&v));
    v.sort_by_key(|n| n.0);
    println!("{:?}", ids(&v));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    println!("{:?}", ids(&v));
    v.sort_unstable_by_key(|n| n.0);
    v.rotate_left(1);
    println!("{:?}", ids(&v));
    let tail = v.split_off(2);
    println!("{:?} {:?}", ids(&v), ids(&tail));
    let mut more = vec![Noisy(9)];
    v.append(&mut more);
    println!("{:?} {}", ids(&v), more.len());
    drop(tail);
    let taken = std::mem::take(&mut v);
    println!("taken {:?} left {}", ids(&taken), v.len());
    drop(swapped);

    let mut w: Vec<Noisy> = (10..20).map(Noisy).collect();
    w.retain(|n| {
        println!("keep {}?", n.0);
        n.0 % 3 != 0
    });
    println!("{:?}", ids(&w));
    w.dedup_by_key(|n| {
        println!("key {}", n.0);
        n.0 / 4
    });
    println!("{:?}", ids(&w));
    w.truncate(3);
    println!("{:?}", ids(&w));
    w.resize_with(5, || Noisy(30));
    w.resize_with(2, || Noisy(31));
    println!("{:?}", ids(&w));
    w.clear();
    println!("cleared {}", w.len());

    let mut deque = std::collections::VecDeque::new();
    deque.push_back(Noisy(40));
    deque.push_back(Noisy(41));
    deque.push_back(Noisy(42));
    deque.push_front(Noisy(39));
    deque.insert(1, Noisy(38));
    let front = deque.pop_front();
    let second = deque.remove(1);
    println!("{:?} {:?}", front.as_ref().map(|n| n.0), second.as_ref().map(|n| n.0));
    drop(front);
    deque.retain(|n| n.0 != 41);
    deque.truncate(1);
    println!("deque {}", deque.len());
    deque.clear();
    println!("end");
}
