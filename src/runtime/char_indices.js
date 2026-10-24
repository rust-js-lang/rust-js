
function $charIndices(s) {
  const indices = [];
  let at = 0;
  for (const c of s) {
    indices.push([at, c]);
    at += $byteLen(c);
  }
  return indices;
}

// `it.as_str()` of a string's `char_indices()`: the text of the `char`s
// it has left (#118).
function $charIndicesStr(it) {
  return it.items.slice(it.at, it.end).map(([, c]) => c).join("");
}

// `it.offset()` of a string's `char_indices()`: where its next `char`
// starts, in UTF-8 bytes, or the string's length past its last, as std's
// front is (#118).
function $charIndicesOffset(it) {
  if (it.at < it.items.length) return it.items[it.at][0];
  const last = it.items[it.items.length - 1];
  return last === undefined ? 0 : last[0] + $byteLen(last[1]);
}
