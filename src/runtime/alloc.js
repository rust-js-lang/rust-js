// The allocator (ADR 0417): an allocation is an array of slots, given as a
// pointer to its first's bytes, which it's cast back from to its items'.
function $alloc(layout) {
  return $ptrBytes($ptr([], 0));
}

// `realloc(p, layout, size)`: the same allocation, as an array grows.
function $realloc(pointer, layout, size) {
  return pointer;
}

// `handle_alloc_error(layout)`: the program's end, as Rust says it.
function $allocError(layout) {
  throw new Error(`memory allocation of ${layout.size} bytes failed`);
}

// `Layout::from_size_align(size, align)`: `Err` where `align` isn't a power
// of two, or `size` rounded up to it is past `isize::MAX`.
function $layout(size, align) {
  const ok = align > 0 && (align & (align - 1)) === 0 && size <= 2 ** 31 - align;
  return ok ? { TAG: "Ok", _0: { size, align } } : { TAG: "Err", _0: undefined };
}

// `{:?}` of a `Layout`.
function $debugLayout(layout) {
  return `Layout { size: ${layout.size}, align: ${layout.align} (1 << ${Math.log2(layout.align)}) }`;
}

// `Vec::from_raw_parts(p, len, cap)`: the array `p` points to the first of,
// `len` long. Of another place, rust-js's error.
function $vecFromRaw(pointer, length) {
  if (pointer === null || !Array.isArray(pointer.of) || pointer.at !== 0) {
    throw new Error("rust-js does not support a `Vec` of what isn't an array's first item, which reads memory by where it is");
  }
  pointer.of.length = length;
  return pointer.of;
}
