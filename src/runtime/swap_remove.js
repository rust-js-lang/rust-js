
function $swapRemove(v, index) {
  if (index >= v.length) {
    throw new Error(`swap_remove index (is ${index}) should be < len (is ${v.length})`);
  }
  const item = v[index];
  v[index] = v[v.length - 1];
  v.pop();
  return item;
}
