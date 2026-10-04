// `Borrow` in generic code, as a map's key lookup is: a dictionary each
// caller gives, std's for a value as itself, a `String` as a `str`, a `Vec`
// or an array as a slice, and the crate's own for its types.

use std::borrow::Borrow;
use std::rc::Rc;

fn contains<K: Borrow<Q>, Q: PartialEq + ?Sized>(keys: &[K], wanted: &Q) -> bool {
    keys.iter().any(|key| key.borrow() == wanted)
}

fn total<V: Borrow<[i32]>>(values: V) -> i32 {
    values.borrow().iter().sum()
}

// A blanket impl over every `K: Borrow<Q>`, as the `equivalent` crate's is.
trait Equivalent<K: ?Sized> {
    fn equivalent(&self, key: &K) -> bool;
}

impl<Q: ?Sized + Eq, K: ?Sized + Borrow<Q>> Equivalent<K> for Q {
    fn equivalent(&self, key: &K) -> bool {
        PartialEq::eq(self, key.borrow())
    }
}

fn position<Q: ?Sized + Equivalent<K>, K>(keys: &[K], wanted: &Q) -> Option<usize> {
    keys.iter().position(|key| wanted.equivalent(key))
}

#[derive(PartialEq, Eq, Debug)]
struct Uuid(u32);

struct Hyphenated(Uuid);

impl Borrow<Uuid> for Hyphenated {
    fn borrow(&self) -> &Uuid {
        &self.0
    }
}

struct Tag {
    name: String,
}

impl Borrow<str> for Tag {
    fn borrow(&self) -> &str {
        &self.name
    }
}

// Ordered by what its key borrows as: std's `sort` calls the crate's `Ord`.
struct Labelled<K>(K);

impl<K: Borrow<str>> PartialEq for Labelled<K> {
    fn eq(&self, other: &Self) -> bool {
        self.0.borrow() == other.0.borrow()
    }
}

impl<K: Borrow<str>> Eq for Labelled<K> {}

impl<K: Borrow<str>> PartialOrd for Labelled<K> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Borrow<str>> Ord for Labelled<K> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.borrow().cmp(other.0.borrow())
    }
}

fn main() {
    let words = vec![String::from("tea"), String::from("milk")];
    println!("{} {}", contains(&words, "milk"), contains(&words, "coffee"));
    println!("{} {}", contains(&[1, 2, 3], &2), contains(&["a", "b"], &"c"));
    let tags = [Tag { name: "red".to_string() }, Tag { name: "blue".to_string() }];
    println!("{} {}", contains(&tags, "blue"), contains(&tags, "green"));
    let ids = [Hyphenated(Uuid(7)), Hyphenated(Uuid(9))];
    println!("{}", contains(&ids, &Uuid(9)));

    println!("{} {} {}", total([1, 2, 3]), total(vec![4, 5]), total(&[6, 7][..]));

    println!("{:?} {:?}", position(&words, "milk"), position(&words, "rum"));
    println!("{:?} {:?}", position(&[5, 6], &6), position(&tags, "red"));
    println!("{:?}", position(&ids, &Uuid(7)));

    let id: &Uuid = ids[1].borrow();
    let name: &str = tags[0].borrow();
    let text: &str = words[0].borrow();
    let (list, boxed, shared) = (vec![1, 2], Box::new(3), Rc::new(String::from("rc")));
    let items: &[i32] = list.borrow();
    let boxed: &i32 = boxed.borrow();
    let shared: &String = shared.borrow();
    println!("{:?} {} {} {:?} {} {}", id, name, text, items, boxed, shared);

    let mut labelled = vec![Labelled(Tag { name: "pear".to_string() }), Labelled(Tag { name: "fig".to_string() })];
    labelled.sort();
    println!("{} {}", labelled[0].0.name, labelled[1].0.name);
}
