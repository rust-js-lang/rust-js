
function $dedupBy(v, same) {
  let n = 0;
  for (const item of v) {
    if (n === 0 || !same(item, v[n - 1])) {
      v[n++] = item;
    }
  }
  v.length = n;
}
