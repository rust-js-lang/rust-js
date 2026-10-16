use std::cell::RefCell;

use smallvec::{SmallVec, smallvec};

thread_local! {
    static DROPS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// A value whose drop is seen, to count each.
#[derive(Clone, Debug)]
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        DROPS.with(|d| d.borrow_mut().push(self.0.to_string()));
    }
}

fn drops() -> String {
    DROPS.with(|d| d.borrow_mut().drain(..).collect::<Vec<_>>().join(","))
}

pub fn report() -> String {
    let mut lines = Vec::new();

    // Inline, then spilled to the heap as it grows past its array.
    let mut v: SmallVec<[i32; 4]> = SmallVec::new();
    for n in [5, 1, 4] {
        v.push(n);
    }
    lines.push(format!("{:?} {} {} {}", v, v.len(), v.inline_size(), v.spilled()));
    v.extend([7, 8, 2]);
    lines.push(format!("{:?} {} {}", v, v.spilled(), v.capacity() >= 6));
    v.insert(1, 9);
    let removed = v.remove(0);
    let popped = v.pop();
    lines.push(format!("{removed} {popped:?} {v:?}"));
    v.retain(|n| *n % 2 == 1);
    v.dedup();
    lines.push(format!("{v:?} {:?}", &v[1..]));
    let drained: Vec<i32> = v.drain(..1).collect();
    lines.push(format!("{drained:?} {v:?}"));
    v.truncate(1);
    v.shrink_to_fit();
    lines.push(format!("{v:?} {}", v.spilled()));

    // From a `Vec`, and back.
    let from: SmallVec<[u8; 2]> = SmallVec::from_vec(vec![1, 2, 3]);
    let small: SmallVec<[u8; 4]> = SmallVec::from_vec(vec![4, 5]);
    lines.push(format!("{from:?} {} {small:?} {}", from.spilled(), small.spilled()));
    lines.push(format!("{:?} {:?}", from.into_vec(), small.into_inner()));

    let mut words: SmallVec<[String; 2]> = smallvec!["a".to_string(), "b".to_string()];
    words.insert_many(1, ["x".to_string(), "y".to_string()]);
    words.resize(5, "z".to_string());
    words.sort();
    lines.push(format!("{words:?} {}", words.join("")));
    let mut other: SmallVec<[String; 2]> = smallvec!["q".to_string()];
    words.append(&mut other);
    lines.push(format!(
        "{} {:?} {}",
        words.len(),
        other,
        words.contains(&"q".to_string())
    ));
    let collected: SmallVec<[usize; 8]> = words.iter().map(|w| w.len()).collect();
    lines.push(format!(
        "{collected:?} {}",
        collected == SmallVec::<[usize; 8]>::from_slice(&[1; 6])
    ));

    // What's dropped, and when: a spilled one's items, what `truncate`
    // leaves, a `Drain` taken in part, and `resize`'s value.
    {
        let mut loud: SmallVec<[Loud; 2]> = SmallVec::new();
        loud.push(Loud("a"));
        loud.push(Loud("b"));
        loud.push(Loud("c"));
        loud.truncate(2);
        lines.push(format!("truncated: {}", drops()));
        let mut drain = loud.drain(..);
        let first = drain.next();
        drop(drain);
        lines.push(format!("drained {first:?}: {}", drops()));
        drop(first);
        loud.resize(2, Loud("r"));
        lines.push(format!("resized {}: {}", loud.len(), drops()));
    }
    lines.push(format!("dropped: {}", drops()));

    lines.join("\n")
}
