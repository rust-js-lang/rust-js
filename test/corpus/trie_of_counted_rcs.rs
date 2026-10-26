// A trie whose nodes hold their children's `Rc`s in a map and their
// parent's `Weak`, as Aho-Corasick's do: each node's drop is a function
// that calls itself for the nodes in its map, and a `Weak` that's
// upgraded counts the `Rc`s, so the last of each drops its node.
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

struct Label(String);

impl Drop for Label {
    fn drop(&mut self) {
        println!("drop {:?}", self.0);
    }
}

struct Node {
    label: Label,
    children: BTreeMap<char, Rc<RefCell<Node>>>,
    parent: Weak<RefCell<Node>>,
}

fn insert(root: &Rc<RefCell<Node>>, word: &str) {
    let mut cur = Rc::clone(root);
    for c in word.chars() {
        let label = Label(format!("{}{c}", cur.borrow().label.0));
        let next = Rc::clone(cur.borrow_mut().children.entry(c).or_insert_with(|| {
            Rc::new(RefCell::new(Node {
                label,
                children: BTreeMap::new(),
                parent: Rc::downgrade(&cur),
            }))
        }));
        cur = next;
    }
}

fn depth(node: &Rc<RefCell<Node>>) -> usize {
    let mut cur = Rc::clone(node);
    let mut depth = 0;
    loop {
        let parent = cur.borrow().parent.upgrade();
        match parent {
            Some(parent) => cur = parent,
            None => break,
        }
        depth += 1;
    }
    depth
}

fn main() {
    let root = Rc::new(RefCell::new(Node {
        label: Label(String::new()),
        children: BTreeMap::new(),
        parent: Weak::new(),
    }));
    for word in ["ab", "ac", "b"] {
        insert(&root, word);
    }
    let a = Rc::clone(&root.borrow().children[&'a']);
    let ab = Rc::clone(&a.borrow().children[&'b']);
    println!("{} {} {}", depth(&root), depth(&a), depth(&ab));
    let taken = a.borrow_mut().children.remove(&'c');
    println!("taken {}", taken.is_some());
    drop(taken);
    drop(a);
    println!("root dropped next");
    drop(root);
    println!("ab's parent is gone: {}", ab.borrow().parent.upgrade().is_none());
}
