// `v.resize(length, item)`: shortened, as `truncate` drops what's past it,
// then `item` dropped, or lengthened with clones of `item`, and `item`
// itself last, as std's is (ADR 0507); room for `length` items of `size`
// bytes each asked first, where it may not fit (ADR 0774). `item` is
// dropped as any of them unwinds, a drop of what's cut off, no room or a
// clone, as std's `resize` owns it till the last; the clones pushed stay
// `v`'s (#898).
function $resize(v, length, item, size, clone, drop) {
  if (length <= v.length) {
    $unwinds(
      () => $truncate(v, length, drop),
      () => drop?.(item),
    );
    drop?.(item);
    return;
  }
  $unwinds(
    () => {
      if (size !== undefined) $roomFor(length, size);
      for (let i = v.length + 1; i < length; i++) v.push(clone === undefined ? item : clone(item));
    },
    () => drop?.(item),
  );
  v.push(item);
}
