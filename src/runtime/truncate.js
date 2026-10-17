// `v.truncate(length)`: what's past it gone, and of items with a destructor,
// each dropped by `drop`, in order, once it's out, as std's are (ADR 0098).
function $truncate(v, length, drop) {
  if (length < v.length) {
    const gone = v.splice(length);
    if (drop !== undefined) {
      for (const item of gone) drop(item);
    }
  }
}
