// `v.extend(items)`: each pushed in turn. Where stepping them throws, a
// panic in a closure, what's left of them closed, dropped, as Rust drops
// the iterator it was given as it unwinds; what it pushed stays, `v`'s
// (#422). `byFold`, of a `TrustedLen` iterator, by its fold, as std's
// `extend_trusted` steps it, which drops what it holds where its fold
// does, a `map`'s function before what it hasn't given (#710).
function $extend(v, items, byFold = false) {
  if (byFold && typeof items.fold === "function") {
    items.fold(undefined, (_, x) => {
      v.push(x);
    });
    return;
  }
  const it = items[Symbol.iterator]();
  try {
    for (let step = it.next(); !step.done; step = it.next()) v.push(step.value);
  } catch (error) {
    it.return?.();
    throw error;
  }
}
