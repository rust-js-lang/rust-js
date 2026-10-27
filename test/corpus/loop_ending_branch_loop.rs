// A loop that ends a branch of an `if` that ends another loop's body: its
// way out is the outer loop's next turn, so it's a `while` of its own test,
// `while (child != null)`, not a `continue bb7` of a label (ADR 0961), as
// React's test selectors walk a fiber's children. A reversed range down to
// 1 is `i > 0`, as a person writes it, and a `for`'s body that ends in a
// branch of `if`s each of which `break`s ends with no `continue`.
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;

struct Fiber {
    tag: u32,
    child: RefCell<Option<Rc<Fiber>>>,
    sibling: RefCell<Option<Rc<Fiber>>>,
}

fn fiber(tag: u32, child: Option<Rc<Fiber>>, sibling: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber {
        tag,
        child: RefCell::new(child),
        sibling: RefCell::new(sibling),
    })
}

fn has_tag(root: &Rc<Fiber>, tag: u32) -> bool {
    let mut stack = vec![(Rc::clone(root), 0)];
    let mut index = 0;
    while index < stack.len() {
        let (fiber, mut depth) = stack[index].clone();
        index += 1;
        if fiber.tag == 7 {
            continue;
        } else {
            depth += 1;
        }

        if fiber.tag == tag {
            println!("found at {depth}");
            return true;
        } else {
            let mut child = fiber.child.borrow().clone();
            while let Some(current) = child {
                stack.push((Rc::clone(&current), depth));
                child = current.sibling.borrow().clone();
            }
        }
    }

    false
}

fn pairs(n: usize) -> usize {
    let mut count = 0;
    for i in (1..n).rev() {
        for j in (0..i).rev() {
            if (i + j) % 3 == 0 {
                count += 1;
            }
        }
    }
    count
}

struct BoundingRect {
    x: Cell<f64>,
    y: Cell<f64>,
    width: Cell<f64>,
    height: Cell<f64>,
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> BoundingRect {
    BoundingRect {
        x: Cell::new(x),
        y: Cell::new(y),
        width: Cell::new(width),
        height: Cell::new(height),
    }
}

// React's findBoundingRects's merge of the rects it found.
fn merged(mut bounding_rects: Vec<BoundingRect>) -> Vec<BoundingRect> {
    for i in (1..bounding_rects.len()).rev() {
        let target_rect = &bounding_rects[i];
        let target_left = target_rect.x.get();
        let target_right = target_left + target_rect.width.get();
        let target_top = target_rect.y.get();
        let target_bottom = target_top + target_rect.height.get();

        for j in (0..i).rev() {
            if i != j {
                let other_rect = &bounding_rects[j];
                let other_left = other_rect.x.get();
                let other_right = other_left + other_rect.width.get();
                let other_top = other_rect.y.get();
                let other_bottom = other_top + other_rect.height.get();

                if target_left >= other_left
                    && target_top >= other_top
                    && target_right <= other_right
                    && target_bottom <= other_bottom
                {
                    // Complete overlapping rects; remove the inner one.
                    bounding_rects.remove(i);
                    break;
                } else if target_left == other_left
                    && target_rect.width.get() == other_rect.width.get()
                    && other_bottom.partial_cmp(&target_top) != Some(Ordering::Less)
                    && other_top.partial_cmp(&target_bottom) != Some(Ordering::Greater)
                {
                    // Adjacent vertical rects; merge them.
                    if other_top > target_top {
                        other_rect
                            .height
                            .set(other_rect.height.get() + (other_top - target_top));
                        other_rect.y.set(target_top);
                    }
                    if other_bottom < target_bottom {
                        other_rect.height.set(target_bottom - other_top);
                    }

                    bounding_rects.remove(i);
                    break;
                } else if target_top == other_top
                    && target_rect.height.get() == other_rect.height.get()
                    && other_right.partial_cmp(&target_left) != Some(Ordering::Less)
                    && other_left.partial_cmp(&target_right) != Some(Ordering::Greater)
                {
                    // Adjacent horizontal rects; merge them.
                    if other_left > target_left {
                        other_rect
                            .width
                            .set(other_rect.width.get() + (other_left - target_left));
                        other_rect.x.set(target_left);
                    }
                    if other_right < target_right {
                        other_rect.width.set(target_right - other_left);
                    }

                    bounding_rects.remove(i);
                    break;
                }
            }
        }
    }

    bounding_rects
}

fn main() {
    let leaf = fiber(5, None, None);
    let hidden = fiber(7, Some(fiber(9, None, None)), Some(leaf));
    let root = fiber(3, Some(fiber(4, None, Some(hidden))), None);
    println!("{} {} {}", has_tag(&root, 5), has_tag(&root, 9), has_tag(&root, 4));
    println!("{} {}", pairs(6), pairs(1));
    let rects = merged(vec![
        rect(0.0, 0.0, 4.0, 4.0),
        rect(1.0, 1.0, 2.0, 2.0),
        rect(0.0, 4.0, 4.0, 2.0),
        rect(9.0, 0.0, 1.0, 1.0),
        rect(8.0, 0.0, 1.0, 1.0),
    ]);
    for r in &rects {
        println!("{} {} {} {}", r.x.get(), r.y.get(), r.width.get(), r.height.get());
    }
}
