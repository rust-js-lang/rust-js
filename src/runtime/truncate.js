// `v.truncate(length)`: what's past it gone, and of items with a destructor,
// each dropped by `drop`, in order, once it's out, as std's are (ADR 0098).
function $truncate(v, length, drop) {
  if (length < v.length) {
    const gone = v.splice(length);
    // The rest still dropped after one panics, as std's `drop_in_place`
    // of the slice goes on (#416).
    if (drop !== undefined) $dropItems(gone, drop);
  }
}
