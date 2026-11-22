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

// A `VecDeque`'s `retain_back(length)`: its last `length` kept, those before
// them dropped in their order, the rest still dropped after one panics, as
// std's `drop_in_place` of each half goes on (#1277).
function $retainBack(v, length, drop) {
  if (length < v.length) {
    const gone = v.splice(0, v.length - length);
    if (drop !== undefined) $dropItems(gone, drop);
  }
}
