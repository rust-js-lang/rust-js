// `v.resize(length, item)`: shortened, as `truncate` drops what's past it,
// then `item` dropped, or lengthened with clones of `item`, and `item`
// itself last, as std's is (ADR 0507); room for `length` items of `size`
// bytes each asked first, where it may not fit, and of none, `item`
// dropped as it unwinds (ADR 0774).
function $resize(v, length, item, size, clone, drop) {
  if (length <= v.length) {
    $truncate(v, length, drop);
    drop?.(item);
    return;
  }
  if (size !== undefined) {
    try {
      $roomFor(length, size);
    } catch (panic) {
      drop?.(item);
      throw panic;
    }
  }
  for (let i = v.length + 1; i < length; i++) v.push(clone === undefined ? item : clone(item));
  v.push(item);
}
