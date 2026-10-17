// `v.resize_with(length, make)`: shortened, as `truncate` drops what's past
// it, or lengthened with what `make` makes.
function $resizeWith(v, length, make, drop) {
  if (length <= v.length) {
    $truncate(v, length, drop);
    return;
  }
  while (v.length < length) v.push(make());
}
