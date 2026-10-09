
function $popIfCell(v, holds) {
  if (v.length === 0) return undefined;
  const last = { value: v[v.length - 1] };
  const popped = holds(last);
  v[v.length - 1] = last.value;
  return popped ? v.pop() : undefined;
}
