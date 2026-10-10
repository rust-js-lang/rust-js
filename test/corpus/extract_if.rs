// `extract_if`: an iterator that, as each item is asked for, asks the
// filter of the next and takes out what it holds of. What it doesn't reach
// stays, and the filter may change what it's given.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList};

fn main() {
    let mut v: Vec<i32> = (1..=10).collect();
    let evens: Vec<i32> = v.extract_if(.., |x| *x % 2 == 0).collect();
    println!("{evens:?} {v:?}");
    let mut w: Vec<i32> = (1..=10).collect();
    for x in w.extract_if(2..8, |x| {
        *x *= 10;
        *x > 40
    }) {
        println!("took {x}");
        if x > 60 {
            break;
        }
    }
    println!("{w:?}");
    let mut names = vec![String::from("ann"), String::from("bob"), String::from("al")];
    let a: Vec<String> = names.extract_if(.., |n| n.starts_with('a')).collect();
    println!("{a:?} {names:?}");
    println!("{}", w.extract_if(1..1, |_| true).count());

    let mut list: LinkedList<i32> = (1..=6).collect();
    let odd: Vec<i32> = list.extract_if(|x| *x % 2 == 1).collect();
    println!("{odd:?} {list:?}");

    let mut scores: HashMap<&str, i32> = HashMap::from([("a", 1), ("b", 20), ("c", 3), ("d", 40)]);
    let mut high: Vec<(&str, i32)> = scores
        .extract_if(|_, v| {
            *v += 1;
            *v > 10
        })
        .collect();
    high.sort();
    let mut left: Vec<_> = scores.into_iter().collect();
    left.sort();
    println!("{high:?} {left:?}");
    let mut set: HashSet<i32> = (1..=6).collect();
    let mut small: Vec<i32> = set.extract_if(|x| *x < 3).collect();
    small.sort();
    println!("{small:?} {}", set.len());

    let mut tree: BTreeMap<i32, String> = (1..=8).map(|i| (i, i.to_string())).collect();
    let taken: Vec<(i32, String)> = tree.extract_if(3..=6, |k, v| {
        v.push('!');
        k % 2 == 0
    }).collect();
    println!("{taken:?} {tree:?}");
    println!("{}", tree.extract_if(6..2, |_, _| true).count());
    let mut keys: BTreeSet<i32> = (1..=8).collect();
    let mut firsts = keys.extract_if(.., |k| *k > 2);
    println!("{:?} {:?}", firsts.next(), firsts.next());
    drop(firsts);
    println!("{keys:?}");
}
