
function $resizeWith(v, length, make) {
  if (length <= v.length) {
    v.length = length;
    return;
  }
  while (v.length < length) v.push(make());
}
