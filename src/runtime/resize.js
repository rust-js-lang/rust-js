
function $resize(v, length, item, clone) {
  if (length <= v.length) {
    v.length = length;
    return;
  }
  for (let i = v.length + 1; i < length; i++) v.push(clone === undefined ? item : clone(item));
  v.push(item);
}
