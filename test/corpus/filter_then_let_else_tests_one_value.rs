// A `let .. else` of a map's value an `Option::filter` keeps, React's
// Flight client's `stopStream`: one value, tested where it's made, `const
// chunk = chunks.get(id); if (chunk == null || chunk.state !== "Ready") {
// return false; }`, named as the program binds it, as upstream writes
// `const chunk = chunks.get(id);` (#1619), not a temporary and the filter's
// result apart.
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
enum State {
    Ready,
    Waiting,
}

struct Chunk {
    state: State,
    id: u32,
}

fn stop(chunks: &HashMap<u32, Chunk>, id: u32) -> Option<u32> {
    let Some(chunk) = chunks.get(&id).filter(|chunk| chunk.state == State::Ready) else {
        return None;
    };
    Some(chunk.id)
}

fn main() {
    let mut chunks = HashMap::new();
    chunks.insert(1, Chunk { state: State::Ready, id: 10 });
    chunks.insert(2, Chunk { state: State::Waiting, id: 20 });
    println!("{:?} {:?} {:?}", stop(&chunks, 1), stop(&chunks, 2), stop(&chunks, 3));
}
