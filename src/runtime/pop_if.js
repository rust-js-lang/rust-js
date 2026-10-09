
function $popIf(v, holds) {
  return v.length !== 0 && holds(v[v.length - 1]) ? v.pop() : undefined;
}
