// A pattern's binding copied right after it, `let parent_object =
// *parent_object;`, is the pattern's own: one destructuring, `const {
// handler, parentObject, key, map, path } = reference;`, as React's
// fulfillReference writes it (#546). Nothing runs between the pattern and
// the copy, so the field read there has the value the copy reads.
use std::rc::Rc;

pub struct Reference {
    pub handler: Rc<String>,
    pub parent_object: u32,
    pub key: String,
    pub map: Vec<u32>,
    pub path: Vec<String>,
    pub extra: bool,
}

pub fn fulfill(reference: &Rc<Reference>) -> usize {
    let Reference {
        handler,
        parent_object,
        key,
        map,
        path,
        ..
    } = &**reference;
    let parent_object = *parent_object;
    handler.len() + parent_object as usize + key.len() + map.len() + path.len()
}

// A copy that's changed after is a `let` of its own, the field's value
// still read where the pattern is.
pub fn bumped(reference: &Reference) -> u32 {
    let Reference {
        parent_object, map, ..
    } = reference;
    let mut parent_object = *parent_object;
    for m in map {
        parent_object += m;
    }
    parent_object
}

fn main() {
    let reference = Rc::new(Reference {
        handler: Rc::new("on".to_string()),
        parent_object: 7,
        key: "key".to_string(),
        map: vec![1, 2],
        path: vec!["a".to_string()],
        extra: true,
    });
    println!("{} {} {}", fulfill(&reference), bumped(&reference), reference.extra);
}
