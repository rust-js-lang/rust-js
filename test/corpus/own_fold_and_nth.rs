// An iterator whose impl writes its own `fold` and `nth`: std's consumers
// that route through them call them, through each adapter whose own does,
// and those that step by `next` don't (#162).
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};

struct Up {
    n: u32,
}

impl Iterator for Up {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        println!("next");
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }

    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, mut f: F) -> B {
        println!("fold");
        (self.n + 1..=3).fold(init, |acc, x| f(acc, x))
    }

    fn nth(&mut self, k: usize) -> Option<u32> {
        println!("nth {k}");
        self.n += k as u32;
        self.next()
    }
}

fn up() -> Up {
    Up { n: 0 }
}

fn total<I: Iterator<Item = u32>>(items: I) -> u32 {
    items.sum()
}

fn third<I: Iterator<Item = u32>>(mut items: I) -> Option<u32> {
    items.nth(2)
}

fn boxed_total(items: Box<dyn Iterator<Item = u32>>) -> u32 {
    items.sum()
}

fn main() {
    println!("sum {}", up().sum::<u32>());
    println!("count {}", up().count());
    println!("last {:?}", up().last());
    up().for_each(|x| println!("each {x}"));
    println!("map fold {}", up().map(|x| x * 10).fold(0, |a, x| a + x));
    println!(
        "filter map sum {}",
        up().filter(|x| x % 2 == 1).map(|x| x + 1).sum::<u32>()
    );
    println!("enumerate {}", up().enumerate().fold(0, |a, (i, x)| a + i as u32 * x));
    println!("max {:?}", up().max());
    println!("min_by_key {:?}", up().min_by_key(|x| *x % 2));
    println!("skip next {:?}", up().skip(2).next());
    println!("skip last {:?}", up().skip(1).last());
    println!("step_by {:?}", up().step_by(2).collect::<Vec<_>>());
    println!("take sum {}", up().take(2).sum::<u32>());
    println!("chain {}", up().chain(up()).fold(0, |a, x| a + x));
    println!("chain last {:?}", up().chain(up()).last());
    println!("flat_map {}", up().flat_map(|x| 0..x).count());
    println!("by_ref {}", up().by_ref().sum::<u32>());
    let text: String = up().map(|x| char::from_digit(x, 10).unwrap()).collect();
    println!("string {text}");
    let map: HashMap<u32, u32> = up().map(|x| (x, x)).collect();
    println!("map {}", map.len());
    let set: HashSet<u32> = up().collect();
    println!("set {}", set.len());
    let list: LinkedList<u32> = up().collect();
    println!("list {list:?}");
    let deque: VecDeque<u32> = up().collect();
    println!("deque {deque:?}");
    println!("vec {:?}", up().collect::<Vec<_>>());
    let (even, odd): (Vec<u32>, Vec<u32>) = up().partition(|x| x % 2 == 0);
    println!("partition {even:?} {odd:?}");
    let (a, b): (Vec<u32>, Vec<u32>) = up().map(|x| (x, x)).unzip();
    println!("unzip {a:?} {b:?}");
    println!("generic sum {}", total(up()));
    println!("generic nth {:?}", third(up()));
    println!("boxed sum {}", boxed_total(Box::new(up())));
    let mut set = HashSet::new();
    set.extend(up());
    println!("extend set {}", set.len());
    let mut text = String::new();
    text.extend(up().map(|x| char::from_digit(x, 10).unwrap()));
    println!("extend string {text}");
    let mut tree = BTreeSet::new();
    tree.extend(up());
    println!("extend tree {tree:?}");
    let mut map = HashMap::new();
    map.extend(up().map(|x| (x, x)));
    println!("extend map {}", map.len());
    let mut sorted = BTreeMap::new();
    sorted.extend(up().map(|x| (x, x)));
    println!("extend sorted {sorted:?}");
    let mut list = LinkedList::new();
    list.extend(up());
    println!("extend list {list:?}");
    let mut deque = VecDeque::new();
    deque.extend(up());
    println!("extend deque {deque:?}");
    let mut heap = BinaryHeap::new();
    heap.extend(up());
    println!("extend heap {}", heap.len());
    let mut vec = vec![0];
    vec.extend(up());
    println!("extend vec {vec:?}");
}
