
function $at(items, index, failure) {
  if (index < 0 || index >= items.length) {
    throw new Error(failure ?? `index out of bounds: the len is ${items.length} but the index is ${index}`);
  }
  return index;
}
