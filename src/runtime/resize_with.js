// `v.resize_with(length, make)`: shortened, as `truncate` drops what's past
// it, or lengthened with what `make` makes, room for `length` items of
// `size` bytes each asked first, where it may not fit (ADR 0774).
function $resizeWith(v, length, make, size, drop) {
  if (length <= v.length) {
    $truncate(v, length, drop);
    return;
  }
  if (size !== undefined) $roomFor(length, size);
  while (v.length < length) v.push(make());
}
