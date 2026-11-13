// The allocator (ADR 0417): an allocation is an array of slots, given as a
// pointer to its first's bytes, which it's cast back from to its items', or
// whose bytes are read and written through it, each slot a byte (#1099): as
// its first use says, the other use after it rust-js's error.
function $alloc(layout) {
  const slots = [];
  return $byteView({ of: slots, at: 0, stride: 1, typed: $ptr(slots, 0), allocated: true });
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

// `{:?}` of a `Layout`, its derived `Debug`'s: its `size` a `usize`'s by
// the `Formatter`'s `options`, `{:5?}`, `{:x?}`; its `align` an
// `Alignment`'s, `{:?} (1 << {:?})` of a `write!` of its own, which no
// options reach; nested where it's `alternate`, `{:#?}`.
function $debugLayout(layout, options) {
  const { size, align } = layout;
  const shown = $formatted($debugInt(String(size), size.toString(16), options), options, true);
  const fields = [`size: ${shown}`, `align: ${align} (1 << ${Math.log2(align)})`];
  return options?.alternate ? $pretty("Layout {", fields, "}") : `Layout { ${fields.join(", ")} }`;
}

// `Vec::from_raw_parts(p, len, cap)`: the array `p` points to the first of,
// `len` long. Of another place, rust-js's error.
function $vecFromRaw(pointer, length) {
  // Of bytes, a `Vec<u8>`'s alone, whose items are the bytes' slots.
  if (pointer?.bytes !== undefined && (pointer.width !== 1 || pointer.signed !== false || pointer.stride !== 1)) {
    throw $unsupported("rust-js does not support a `Vec` of what isn't an array's first item, which reads memory by where it is");
  }
  if (pointer?.bytes !== undefined) $allocUsed(pointer, "bytes");
  if (pointer === null || !Array.isArray(pointer.of) || pointer.at !== 0) {
    throw $unsupported("rust-js does not support a `Vec` of what isn't an array's first item, which reads memory by where it is");
  }
  $setLen(pointer.of, length);
  return pointer.of;
}
