
// `items[index]`, checked as a slice's index is; a `VecDeque`'s gives its
// own message, `failure`.
function $index(items, index, raisedAt, failure) {
  if (index < 0 || index >= items.length) {
    throw $stringPanic(failure ?? `index out of bounds: the len is ${items.length} but the index is ${index}`, raisedAt);
  }
  return items[index];
}
