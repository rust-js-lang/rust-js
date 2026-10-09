
function $dedupByCells(v, same) {
  let n = 0;
  for (let i = 0; i < v.length; i++) {
    if (n === 0) {
      v[n++] = v[i];
      continue;
    }
    const a = { value: v[i] };
    const b = { value: v[n - 1] };
    const duplicate = same(a, b);
    v[n - 1] = b.value;
    if (!duplicate) {
      v[n++] = a.value;
    }
  }
  v.length = n;
}
