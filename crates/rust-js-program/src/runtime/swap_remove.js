
function $swapRemove(v, index) {
  if (index >= v.length) {
    throw $stringPanic(`swap_remove index (is ${index}) should be < len (is ${v.length})`, $stdSites.swapRemove[0]);
  }
  const item = v[index];
  v[index] = v[v.length - 1];
  v.pop();
  return item;
}
