// A loop inside another, whose way out MIR wrote in a `catch` of its own,
// what the program wrote after the inner loop, a call a borrow tracked at
// run time unborrows past: that `catch` says no place, and is where what
// it holds is written, after the inner loop. So it's after it, `while
// (chunkIdx < nextChild.index) { .. } r = flushSegment(..);`, not `bb7:
// while (..) { while (true) { if (!(..)) { r = flushSegment(..); continue
// bb7; } .. } }` (rd499). React's Fizz flushSubtree.
use std::cell::RefCell;
use std::rc::Rc;

struct Segment {
    index: f64,
    chunks: RefCell<Vec<u32>>,
    children: RefCell<Vec<Rc<Segment>>>,
}

fn segment(index: f64, chunks: Vec<u32>, children: Vec<Rc<Segment>>) -> Rc<Segment> {
    Rc::new(Segment {
        index,
        chunks: RefCell::new(chunks),
        children: RefCell::new(children),
    })
}

fn write_chunk(out: &mut Vec<u32>, chunk: u32) {
    out.push(chunk);
}

fn flush_segment(out: &mut Vec<u32>, segment: &Segment) -> bool {
    out.push(1000);
    segment.children.borrow_mut().clear();
    segment.chunks.borrow_mut().push(3);
    segment.index > 1.0
}

fn flush_subtree(out: &mut Vec<u32>, segment: &Segment) -> bool {
    let mut r = true;
    let chunks = segment.chunks.borrow();
    let mut chunk_idx = 0;
    let children = segment.children.borrow();
    for next_child in children.iter() {
        // Write all the chunks up until the next child.
        while (chunk_idx as f64) < next_child.index {
            write_chunk(out, chunks[chunk_idx]);
            chunk_idx += 1;
        }
        r = flush_segment(out, next_child);
    }
    // Finally just write all the remaining chunks
    while chunk_idx < chunks.len() {
        write_chunk(out, chunks[chunk_idx]);
        chunk_idx += 1;
    }
    r
}

// An inner loop the program leaves by a `break` as well: what's after it
// is reached from both, and stays after it.
fn flush_until(out: &mut Vec<u32>, segment: &Segment, stop: u32) -> usize {
    let chunks = segment.chunks.borrow();
    let mut chunk_idx = 0;
    let children = segment.children.borrow();
    for next_child in children.iter() {
        while (chunk_idx as f64) < next_child.index {
            if chunks[chunk_idx] == stop {
                break;
            }
            write_chunk(out, chunks[chunk_idx]);
            chunk_idx += 1;
        }
        flush_segment(out, next_child);
    }
    chunk_idx
}

fn main() {
    let root = segment(
        0.0,
        vec![1, 2, 3, 4],
        vec![segment(1.0, vec![], vec![]), segment(3.0, vec![], vec![])],
    );
    let mut out = Vec::new();
    let r = flush_subtree(&mut out, &root);
    println!("{r} {out:?}");

    let root = segment(0.0, vec![5, 6, 7], vec![segment(2.0, vec![], vec![])]);
    let mut out = Vec::new();
    let r = flush_subtree(&mut out, &root);
    println!("{r} {out:?}");

    let root = segment(
        0.0,
        vec![1, 9, 3, 4],
        vec![segment(1.0, vec![], vec![]), segment(4.0, vec![], vec![])],
    );
    let mut out = Vec::new();
    let at = flush_until(&mut out, &root, 9);
    println!("{at} {out:?}");
}
