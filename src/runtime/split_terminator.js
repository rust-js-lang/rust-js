// `s.split_terminator(p)`: `split`'s pieces, without an empty last one.
function $splitTerminator(s, p) {
  const parts = $split(s, p);
  if (parts.at(-1) === "") parts.pop();
  return parts;
}
