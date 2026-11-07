// An item read by its index is read as written, `$borrow(segments)[0]`,
// never a pattern, `const [segment] = $borrow(segments);` (ADR 0598,
// amended): React's flushSegment reads `completedSegments[0]` past a
// guard that its length is 1, which the ranges know, so the index needs
// no check. The cell's borrows are counted, as one is held across a call
// that borrows it again. Each read is reached with one segment, with two,
// whose guard panics and is caught, and with none.
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;

struct Boundary {
    completed_segments: RefCell<Vec<Rc<String>>>,
}

fn content_segment(boundary: &Boundary) -> Rc<String> {
    let completed_segments = &boundary.completed_segments;
    if completed_segments.borrow().len() != 1 {
        panic!("A previously unvisited boundary must have exactly one root segment.");
    }
    let content_segment = Rc::clone(&completed_segments.borrow()[0]);
    content_segment
}

fn first_and_last(boundary: &Boundary) -> (u32, u32) {
    let segments = &boundary.completed_segments;
    if segments.borrow().len() < 2 {
        panic!("two segments at least");
    }
    let first = segments.borrow()[0].len() as u32;
    let last = segments.borrow()[1].len() as u32;
    (first, last)
}

fn flushed(boundary: &Boundary, flush: &dyn Fn(&Boundary) -> usize) -> usize {
    let held = boundary.completed_segments.borrow();
    flush(boundary) + held.len()
}

fn main() {
    let boundary = Boundary {
        completed_segments: RefCell::new(vec![Rc::new(String::from("root"))]),
    };
    let total = flushed(&boundary, &|b| content_segment(b).len());
    println!("{total} {}", content_segment(&boundary));
    boundary
        .completed_segments
        .borrow_mut()
        .push(Rc::new(String::from("late")));
    let caught = panic::catch_unwind(panic::AssertUnwindSafe(|| content_segment(&boundary)));
    println!("two segments: {}", caught.is_err());
    let (first, last) = first_and_last(&boundary);
    println!("{first} {last}");
    boundary.completed_segments.borrow_mut().clear();
    let caught = panic::catch_unwind(panic::AssertUnwindSafe(|| first_and_last(&boundary)));
    println!("none: {}", caught.is_err());
}
