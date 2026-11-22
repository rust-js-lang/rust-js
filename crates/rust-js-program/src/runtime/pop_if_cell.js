// `v.pop_if(holds)` of items a `&mut` to is a cell: the last given `holds`
// as `{ value }`, written back as `holds` returns or unwinds, so what it
// wrote is what's popped or kept (#906).
function $popIfCell(v, holds) {
  if (v.length === 0) return undefined;
  const last = { value: v[v.length - 1] };
  let popped;
  try {
    popped = holds(last);
  } finally {
    v[v.length - 1] = last.value;
  }
  return popped ? v.pop() : undefined;
}
