// `as_mut()` of an `Option` a call gave a `&mut` to, `child_mut(left)`'s,
// a handle on its place: through it, a `Some`'s box changed in place and
// one replaced, as an AVL tree's `rebalance` rotates its heavy child.
struct Node {
    value: u8,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

impl Node {
    fn leaf(value: u8) -> Option<Box<Node>> {
        Some(Box::new(Node {
            value,
            left: None,
            right: None,
        }))
    }

    fn child_mut(&mut self, left: bool) -> &mut Option<Box<Node>> {
        if left {
            &mut self.left
        } else {
            &mut self.right
        }
    }

    fn bump_child(&mut self, left: bool) -> bool {
        match self.child_mut(left).as_mut() {
            Some(child) => {
                child.value += 10;
                true
            }
            None => false,
        }
    }

    fn replace_child(&mut self, left: bool, value: u8) {
        if let Some(child) = self.child_mut(left).as_mut() {
            **child = Node {
                value,
                left: None,
                right: Node::leaf(value + 1),
            };
        }
    }
}

fn main() {
    let mut n = Node {
        value: 1,
        left: Node::leaf(2),
        right: None,
    };
    println!("{} {}", n.bump_child(true), n.bump_child(false));
    n.replace_child(true, 7);
    let left = n.left.as_ref().unwrap();
    println!("{} {} {}", n.value, left.value, left.right.as_ref().unwrap().value);
}
