// Cells', locks', a `VecDeque`'s, `char`'s and arrays' other methods, as
// std's are (ADR 0327).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};

fn main() {
    let mut count = Cell::new(1);
    *count.get_mut() += 1;
    let other = Cell::new(10);
    count.swap(&other);
    count.update(|x| x * 2);
    println!("{} {}", count.get(), other.get());
    let mut list = RefCell::new(vec![1]);
    list.get_mut().push(2);
    let more = RefCell::new(vec![9]);
    list.swap(&more);
    println!("{:?} {:?}", list.borrow(), more.borrow());
    let lock = Mutex::new(1);
    let rw = RwLock::new(2);
    lock.clear_poison();
    println!("{} {}", lock.is_poisoned(), rw.is_poisoned());

    let mut deque: VecDeque<i32> = (1..=6).collect();
    println!("{:?} {:?}", deque.get(1), deque.get(9));
    if let Some(x) = deque.get_mut(0) {
        *x += 100;
    }
    *deque.front_mut().unwrap() += 1;
    *deque.back_mut().unwrap() *= 10;
    println!("{deque:?} {:?}", deque.range(1..3).collect::<Vec<_>>());
    println!("{:?} {:?} {deque:?}", deque.swap_remove_back(1), deque.swap_remove_front(3));
    deque.retain_mut(|x| {
        *x += 1;
        *x % 2 == 0
    });
    println!("{deque:?} {}", deque.partition_point(|&x| x < 50));
    println!("{:?} {:?} {deque:?}", deque.pop_front_if(|x| *x > 100), deque.pop_back_if(|x| *x > 100));

    for c in ['a', '\n', '\'', 'é', '\u{1F600}', '\u{7f}'] {
        println!("{} {} {}", c.escape_default(), c.escape_debug(), c.escape_unicode());
    }
    println!("{} {}", "a\tb\"'é".escape_default(), "a\tb\"'é\u{301}".escape_debug());
    println!("{}", "hé".escape_unicode().to_string());
    let mut buffer = [0u8; 4];
    let text = 'é'.encode_utf8(&mut buffer);
    println!("{text} {}", text.len());
    println!("{buffer:?}");
    let decoded: Vec<_> = char::decode_utf16([104, 0xD83D, 0xDE00, 0xD800, 105]).map(|r| r.map_err(|e| e.unpaired_surrogate())).collect();
    println!("{decoded:?}");
    let mut letter = 'q';
    letter.make_ascii_uppercase();
    println!("{letter}");

    let mut pair = [1, 2];
    let refs = pair.each_ref();
    println!("{:?}", refs.map(|x| x * 2));
    for x in pair.each_mut() {
        *x += 1;
    }
    println!("{pair:?}");
}
