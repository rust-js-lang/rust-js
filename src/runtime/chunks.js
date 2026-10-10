// `chunks(size)`: each `size` items, the last fewer, as `cut` cuts them:
// copies, or views of `chunks_mut`'s (ADR 0335).
function $chunks(v, size, cut = (items, start, end) => items.slice(start, end)) {
  if (size === 0) {
    throw new Error("chunk size must be non-zero");
  }
  return Array.from({ length: Math.ceil(v.length / size) }, (_, i) => cut(v, i * size, Math.min(i * size + size, v.length)));
}
