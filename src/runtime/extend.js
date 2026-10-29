// `v.extend(items)`: each pushed in turn. Where stepping them throws, a
// panic in a closure, what's left of them closed, dropped, as Rust drops
// the iterator it was given as it unwinds; what it pushed stays, `v`'s
// (#422).
function $extend(v, items) {
  const it = items[Symbol.iterator]();
  try {
    for (let step = it.next(); !step.done; step = it.next()) v.push(step.value);
  } catch (error) {
    it.return?.();
    throw error;
  }
}
