// `v.resize(length, item)`: shortened, as `truncate` drops what's past it,
// then `item` dropped, or lengthened with clones of `item`, and `item`
// itself last, as std's is (ADR 0507).
function $resize(v, length, item, clone, drop) {
  if (length <= v.length) {
    $truncate(v, length, drop);
    drop?.(item);
    return;
  }
  for (let i = v.length + 1; i < length; i++) v.push(clone === undefined ? item : clone(item));
  v.push(item);
}
