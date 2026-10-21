// A raw pointer is a handle on its place, `{ value }`, one for each place
// (ADR 0407): `object[key]`'s, so two pointers to it are `===`, and `*p` is
// `p.value`. `of` and `at` say where it is, which `add` steps along an array;
// `offset`, a field's byte offset in its object, says where its address is
// (ADR 0562); an item's is its index times its size.
const $places = new WeakMap();
// Each object that's an array's item a handle was made on, to that handle:
// `$ptrTo` of the object is the item's place (ADR 0562).
const $items = new WeakMap();

function $ptr(object, key, offset) {
  let handles = $places.get(object);
  if (handles === undefined) $places.set(object, (handles = new Map()));
  let handle = handles.get(key);
  if (handle === undefined) {
    const item = Array.isArray(object) && typeof key === "number" ? object[key] : undefined;
    if (item !== null && typeof item === "object") $items.set(item, { of: object, at: key });
    handle = {
      of: object,
      at: key,
      offset,
      get value() {
        return $slot(object, key);
      },
      set value(value) {
        object[key] = value;
      },
    };
    handles.set(key, handle);
  }
  return handle;
}

// An array's slots past its length that `Vec::set_len` shortened it by
// (ADR 0420): they stay where they were, as Rust's memory does, read
// through a pointer until they're written again.
const $spares = new WeakMap();

// What's at `object[key]`: of an array past its length, what's left there.
function $slot(object, key) {
  return key >= object.length ? $spares.get(object)?.[key] : object[key];
}

// The `count` items of an array from `start`, a copy, past its length what's
// left there.
function $slots(items, start, count) {
  if (start + count <= items.length) return items.slice(start, start + count);
  return Array.from({ length: count }, (_, i) => $slot(items, start + i));
}

// `v.set_len(n)`, `n` long: what's past it is left where it was, and what
// it's lengthened over is what was written there, or left.
function $setLen(v, n) {
  let spare = $spares.get(v);
  if (n < v.length) {
    if (spare === undefined) $spares.set(v, (spare = []));
    for (let i = n; i < v.length; i++) spare[i] = v[i];
  }
  v.length = n;
  spare?.forEach((item, i) => {
    if (i < n && !(i in v)) v[i] = item;
  });
}

// A pointer to an object a `&mut` is, whose place JS doesn't name: one for
// the object, whose `*p = v` makes it `v` in place, as `*r = v` does.
const $objects = new WeakMap();

function $ptrTo(object) {
  if (object === null || (typeof object !== "object" && typeof object !== "function")) {
    throw new Error("rust-js does not support a pointer to a value JS holds as itself, a copy, which reads memory by where it is");
  }
  let handle = $objects.get(object);
  if (handle === undefined) {
    handle = {
      of: object,
      at: undefined,
      offset: 0,
      get value() {
        return object;
      },
      set value(value) {
        $assign(object, value);
      },
    };
    $objects.set(object, handle);
  }
  return handle;
}

// `p.add(n)`, `p.offset(n)`, `p.sub(n)`: the item `n` along an array a
// pointer is into; of another, where JS has no place, rust-js's error.
function $ptrAdd(pointer, count) {
  if (pointer !== null && pointer.bytes !== undefined) {
    return $byteView({ ...pointer, at: pointer.at + count * pointer.width });
  }
  if (pointer === null || !Array.isArray(pointer.of) || pointer.addr !== undefined) {
    throw new Error("rust-js does not support stepping a pointer that isn't into an array, which reads memory by where it is");
  }
  return $ptr(pointer.of, pointer.at + count);
}

// `p.offset_from(q)`: how many items `p` is past `q`, along one array. Of
// two, which Rust leaves undefined, rust-js's error.
function $ptrOffsetFrom(p, q) {
  if (p === q) return 0;
  if (p?.bytes !== undefined && q?.bytes !== undefined && $bytesOf(p) === $bytesOf(q)) {
    return (p.at - q.at) / p.width;
  }
  if (p.of !== q.of || !Array.isArray(p.of)) {
    throw new Error("rust-js does not support the distance between pointers to two places, which reads memory by where it is");
  }
  return p.at - q.at;
}

// `ptr::replace(p, v)`: what it pointed to, and `v` there now.
function $ptrReplace(pointer, value) {
  const old = pointer.value;
  pointer.value = value;
  return old;
}

// `ptr::swap(a, b)`: what each points to, the other's.
function $ptrSwap(a, b) {
  const held = a.value;
  a.value = b.value;
  b.value = held;
}

// `{:p}` and `{:?}` of a pointer: its address (ADR 0562), `0x0` of `null`.
function $ptrShown(pointer, size) {
  return `0x${$addr(pointer, size).toString(16)}`;
}

// A pointer's address (ADR 0562): a 32-bit number, as wasm32's `usize` is,
// exact within its allocation, the JS array, object, cell or function a
// handle is on: the allocation's base plus the handle's byte offset, an
// item's its index times `size`, the pointee's. A base is given when an
// address is first asked of the allocation, the next free multiple of 4096,
// which reserves the allocation's bytes, rounded up to 4096, and never
// gives them back: no two live allocations overlap, every base is aligned
// past any type's alignment, and addresses rise in the order they're
// asked. Asking exposes the allocation, as Rust's `expose_provenance` does:
// `$ptrFrom` finds a place in it by its address.
const $ranges = new WeakMap();
const $exposed = [];
const $exposedPlaces = new Map();
let $nextBase = 0x10000;

// The allocation a pointer is into, and the handle its address is of: a
// `$ptrTo` of an object that's an array's item, the array's item's.
const $strides = new WeakMap();

function $allocationOf(pointer) {
  if (pointer.bytes !== undefined) return pointer.plain !== undefined ? $allocationOf(pointer.plain) : $bytesOf(pointer);
  return $placed(pointer).of ?? pointer;
}

function $placed(pointer) {
  if (pointer.at === undefined && pointer.of !== undefined) {
    const item = $items.get(pointer.of);
    if (item !== undefined) return $ptr(item.of, item.at);
  }
  return pointer;
}

function $addresses(allocation, bytes) {
  let range = $ranges.get(allocation);
  if (range === undefined) {
    const length = Math.max(4096, Math.ceil(bytes / 4096) * 4096);
    if ($nextBase + length > 0x1_0000_0000) {
      throw new Error("rust-js does not support more addresses than wasm32 has: the address space is used up");
    }
    range = { base: $nextBase, end: $nextBase + length, of: allocation, stride: undefined };
    $nextBase += length;
    $ranges.set(allocation, range);
    $exposed.push(range);
  }
  return range;
}

// A handle's byte offset in its allocation, given the range's stride of an
// array's items.
function $offsetIn(pointer, range) {
  if (pointer.bytes !== undefined) {
    return pointer.at + (pointer.plain !== undefined ? $offsetIn(pointer.plain, range) : 0);
  }
  if (pointer.of === undefined || pointer.at === undefined) return 0;
  if (Array.isArray(pointer.of) && typeof pointer.at === "number") return pointer.at * range.stride;
  if (pointer.offset !== undefined) return pointer.offset;
  throw new Error("rust-js does not support the address of a field of a type its caller lays out, which reads memory by where it is");
}

function $addr(pointer, size) {
  if (pointer === null) return 0;
  if (typeof pointer !== "object" && typeof pointer !== "function") {
    throw new Error("rust-js does not support the address of a value JS holds as itself, a copy, which reads memory by where it is");
  }
  if (pointer.addr !== undefined) return pointer.addr;
  if (pointer.bytes !== undefined && pointer.opaque) return $addr(pointer.typed, size);
  pointer = $placed(pointer);
  const allocation = $allocationOf(pointer);
  const view = pointer.bytes !== undefined && pointer.plain === undefined;
  const items = Array.isArray(allocation) && ((pointer.of !== undefined && typeof pointer.at === "number") || view);
  const stride = view ? pointer.stride : size;
  const range = $addresses(allocation, items ? allocation.length * stride : 0);
  if (items && range.stride === undefined) range.stride = stride;
  const addr = range.base + $offsetIn(pointer, range);
  if (!items) $exposedPlaces.set(addr, pointer);
  return addr;
}

// A pointer with no place: an address, and the provenance of the pointer
// it was made from, if any, which `$ptrAt` may bring it back to.
function $ptrNone(addr, provenance) {
  return {
    addr,
    provenance,
    get value() {
      throw new Error(`rust-js does not support reading through a pointer to no place, at 0x${addr.toString(16)}, which reads memory by where it is`);
    },
    set value(value) {
      throw new Error(`rust-js does not support writing through a pointer to no place, at 0x${addr.toString(16)}, which reads memory by where it is`);
    },
  };
}

// The place at `addr` in `range`, if one is there: an item of an array at a
// boundary of `size`, or a place that exposed `addr`.
function $placeAt(addr, range, size) {
  const place = $exposedPlaces.get(addr);
  if (place !== undefined) return place;
  const of = range.of;
  if (Array.isArray(of) && size > 0 && (addr - range.base) % size === 0) return $ptr(of, (addr - range.base) / size);
  return undefined;
}

// The exposed range `addr` is in, by binary search: bases rise.
function $addressesAt(addr) {
  let low = 0;
  let high = $exposed.length;
  while (low < high) {
    const mid = (low + high) >> 1;
    if ($exposed[mid].end <= addr) low = mid + 1;
    else high = mid;
  }
  const range = $exposed[low];
  return range !== undefined && range.base <= addr ? range : undefined;
}

// `n as *const T`, `with_exposed_provenance(n)` (ADR 0562): the place at
// `n` in an exposed allocation, `size` its items', else a pointer with no
// place, as Rust's strict provenance has it; `0`'s is `null`.
function $ptrFrom(addr, size) {
  addr >>>= 0;
  if (addr === 0) return null;
  const range = $addressesAt(addr);
  return (range && $placeAt(addr, range, size)) ?? $ptrNone(addr, range?.of === undefined ? null : $ptrTo(range.of));
}

// `p.with_addr(n)`, `map_addr`, `wrapping_add` and `wrapping_byte_add`
// (ADR 0562): a pointer with `p`'s provenance at `n`, a 32-bit address as
// wasm32's `usize` wraps: the place there, where its allocation has one,
// else a pointer with no place that remembers it.
function $ptrAt(pointer, addr, size) {
  addr >>>= 0;
  if (pointer === null) return $ptrFrom(addr, size);
  const provenance = pointer.addr === undefined ? pointer : pointer.provenance;
  if (provenance === null) return addr === 0 ? null : $ptrNone(addr, null);
  if (pointer.addr === undefined && addr === $addr(pointer, size)) return pointer;
  const range = $ranges.get($allocationOf(provenance));
  if (range !== undefined && range.base <= addr && addr < range.end) {
    const place = $placeAt(addr, range, size);
    if (place !== undefined) return place;
  }
  return addr === 0 ? null : $ptrNone(addr, provenance);
}

// `p == q` (ADR 0562): the same place, or the same address; `null`'s is 0.
function $ptrEq(p, q) {
  if (p === q) return true;
  if (p === null || q === null) return false;
  if (p.addr !== undefined && q.addr !== undefined) return p.addr === q.addr;
  if (p.addr === undefined && q.addr === undefined) {
    const [a, b] = [$placed(p), $placed(q)];
    if (a === b) return true;
    if ($allocationOf(a) !== $allocationOf(b)) return false;
    // Two of one kind, by where they are; a byte view and a handle, by the
    // offset both are at, where the array's stride is known.
    if ((a.bytes !== undefined) === (b.bytes !== undefined)) return a.at === b.at && a.offset === b.offset;
    const [bytes, handle] = a.bytes !== undefined ? [a, b] : [b, a];
    const stride = $strides.get($allocationOf(handle));
    return stride !== undefined && typeof handle.at === "number" && handle.at * stride === bytes.at;
  }
  const [handle, addr] = p.addr === undefined ? [$placed(p), q.addr] : [$placed(q), p.addr];
  const range = $ranges.get($allocationOf(handle));
  return range !== undefined && range.base + $offsetIn(handle, range) === addr;
}

// `p < q`, `p.cmp(&q)`: by address.
function $ptrCmp(p, q, size) {
  return $cmp($addr(p, size), $addr(q, size));
}

// `ptr::copy(src, dst, n)`, `copy_nonoverlapping`: the `n` items from where
// `src` points to where `dst` does, as `memmove` copies them, each `copy`'d
// where a `Copy` value is an object something changes; of one array, its
// `copyWithin`.
function $ptrCopy(src, dst, count, copy) {
  if (count === 0) return;
  if (src.bytes !== undefined || dst.bytes !== undefined) {
    const [from, to] = [$asBytes(src), $asBytes(dst)];
    const read = Array.from({ length: count }, (_, i) => $byteAt(from, i));
    for (let i = 0; i < count; i++) $byteSet(to, i, read[i]);
    return;
  }
  if (count === 1) {
    dst.value = copy === undefined ? src.value : copy(src.value);
    return;
  }
  if (!Array.isArray(src.of) || !Array.isArray(dst.of)) {
    throw new Error("rust-js does not support copying items but between arrays, which reads memory by where it is");
  }
  if (src.of === dst.of && copy === undefined && Math.max(src.at, dst.at) + count <= src.of.length) {
    src.of.copyWithin(dst.at, src.at, src.at + count);
    return;
  }
  const items = $slots(src.of, src.at, count);
  for (let i = 0; i < count; i++) dst.of[dst.at + i] = copy === undefined ? items[i] : copy(items[i]);
}

// `slice::from_raw_parts(p, n)`: the `n` items from where `p` points, a
// copy, as a shared subslice is, or all of an array, the array itself.
function $ptrSlice(pointer, count) {
  if (pointer.bytes !== undefined) return Array.from({ length: count }, (_, i) => $byteAt(pointer, i));
  const [items, start] = $ptrItems(pointer, count);
  return start === 0 && count === items.length ? items : $slots(items, start, count);
}

// `slice::from_raw_parts_mut(p, n)`: a view of the `n` items from where `p`
// points (ADR 0335), or all of an array, the array itself.
function $ptrSliceMut(pointer, count) {
  // A byte slice's own bytes write through; another's, read as bytes, don't.
  if (pointer.bytes !== undefined) {
    if (pointer.plain === undefined && pointer.stride === 1 && pointer.width === 1) {
      return $view(pointer.of, pointer.at, pointer.at + count);
    }
    throw new Error("rust-js does not support a mutable slice of another type's bytes, which reads memory by where it is");
  }
  const [items, start] = $ptrItems(pointer, count);
  if (start + count > items.length && $spares.has(items)) {
    throw new Error("rust-js does not support a mutable slice past a vector's length, over what's left there, which reads memory by where it is");
  }
  return start === 0 && count === items.length ? items : $view(items, start, start + count);
}

// The array a pointer is into, and its index there, for `count` items from
// it: none, of no array but one place, is an array of it.
function $ptrItems(pointer, count) {
  if (Array.isArray(pointer.of)) return [pointer.of, pointer.at];
  if (count <= 1) return [count === 0 ? [] : [pointer.value], 0];
  throw new Error("rust-js does not support a slice of items that aren't an array's, which reads memory by where it is");
}

// A pointer to an array's first item, as one to the whole of it, of
// `length` items: arrayvec's `ptr::read(p as *const [T; CAP])`. Of another
// item, or of fewer, it would be memory by where it is.
function $ptrWhole(pointer, length) {
  if (Array.isArray(pointer.of) && pointer.at === 0 && pointer.of.length === length) return $ptrTo(pointer.of);
  throw new Error("rust-js does not support a pointer to part of an array as one to an array, which reads memory by where it is");
}

// A pointer as one to another type, in generic code where whether JS has it
// at the same place depends on the type, which reads memory by where it is.
function $ptrRecast(pointer) {
  throw new Error("rust-js does not support a pointer read as another type's, which reads memory by where it is");
}

// A whole array's pointer as one to its first item, of a type parameter's
// every impl of its trait is an array of the item, smallvec's `*const A` as
// `*const A::Item` (ADR 0417).
function $ptrFirst(pointer) {
  return pointer === null ? null : $ptr(pointer.value, 0);
}

// A pointer to bytes (ADR 0562): a view of plain data's bytes, as wasm32
// holds them, little-endian. Of an array of integers, the item each byte
// falls in, `stride` bytes each, read and patched in place, no copy; of a
// value with a codec, a struct's or a field's, its bytes `encode`d where
// read and `decode`d back where written. `at` is the view's byte offset,
// `width` the bytes one read is, a `u32`'s 4, `signed` how they're read,
// or `"bytes"` for a `[u8; N]`. Of what has no codec, an allocation's slots
// (ADR 0417), the view is cast back only, `$ptrTyped`.
function $ptrBytes(pointer, stride, encode, decode) {
  if (pointer === null || pointer.bytes !== undefined) return pointer;
  if (stride === undefined) return $byteView({ opaque: true, typed: pointer });
  if (Array.isArray(pointer.of) && typeof pointer.at === "number") {
    $strides.set(pointer.of, stride);
    return $byteView({ of: pointer.of, at: pointer.at * stride, stride, typed: pointer });
  }
  return $byteView({ plain: pointer, at: 0, stride, encode, decode });
}

function $byteView(view) {
  view.bytes = true;
  view.width ??= 1;
  view.signed ??= false;
  return Object.defineProperty(view, "value", {
    get() {
      if (this.opaque) {
        throw new Error("rust-js does not support reading or writing bytes, which JS has none of, through a pointer to another type");
      }
      if (this.signed === "bytes") return Array.from({ length: this.width }, (_, i) => $byteAt(this, i));
      return $readInt(this, this.width, this.signed);
    },
    set(value) {
      if (this.opaque) {
        throw new Error("rust-js does not support reading or writing bytes, which JS has none of, through a pointer to another type");
      }
      if (this.signed === "bytes") {
        for (let i = 0; i < this.width; i++) $byteSet(this, i, value[i]);
        return;
      }
      $writeInt(this, this.width, value);
    },
  });
}

// A pointer to `u8`s as the view of their bytes it is: a view already; or
// `$ptr(array, i)` into a byte array, whose items are its bytes, or one
// `u8`'s handle, the byte its value is.
function $asBytes(pointer) {
  if (pointer === null || pointer.bytes !== undefined) return pointer;
  return $ptrBytes(pointer, 1, (value) => [value], (bytes) => bytes[0]);
}

// `p.cast::<u32>()` of a pointer to bytes: the same bytes, read `width` at
// a time, `signed` so, or as a `[u8; width]`.
function $ptrRetyped(pointer, width, signed) {
  if (pointer === null) return null;
  return $byteView({ ...$asBytes(pointer), width, signed });
}

// The array a view's bytes are of: its own, or the one encoded.
function $bytesOf(view) {
  return view.plain !== undefined ? view.plain : view.of;
}

// The byte `i` past where `view` is.
function $byteAt(view, i) {
  const at = view.at + i;
  if (view.plain !== undefined) return view.encode(view.plain.value)[at];
  if (view.stride === 1) return $slot(view.of, at);
  const item = Math.floor(at / view.stride);
  return $toBytes($slot(view.of, item), view.stride, true)[at % view.stride];
}

function $byteSet(view, i, byte) {
  const at = view.at + i;
  if (view.plain !== undefined) {
    const bytes = view.encode(view.plain.value);
    bytes[at] = byte;
    view.plain.value = view.decode(bytes);
    return;
  }
  if (view.stride === 1) {
    view.of[at] = byte;
    return;
  }
  const item = Math.floor(at / view.stride);
  const bytes = $toBytes($slot(view.of, item), view.stride, true);
  bytes[at % view.stride] = byte;
  view.of[item] = $fromBytes(bytes, view.stride, true, false);
}

// `read_unaligned` of a `width`-byte integer at a byte pointer: the bytes
// there, little-endian, `signed` as the type is; `write_unaligned` the
// reverse.
function $readInt(view, width, signed) {
  if (width === 1 && view.plain === undefined && view.stride === 1 && !signed) return $slot(view.of, view.at);
  const bytes = Array.from({ length: width }, (_, i) => $byteAt(view, i));
  return $fromBytes(bytes, width, true, signed);
}

function $writeInt(view, width, value) {
  const bytes = width === 1 ? [value] : $toBytes(value, width, true);
  for (let i = 0; i < width; i++) $byteSet(view, i, bytes[i]);
}

// A pointer to bytes as one to a type: what was cast to bytes, or the
// array's item its bytes start at. Of bytes that are bytes alone, rust-js's
// error.
function $ptrTyped(view) {
  if (view === null || view.bytes === undefined) return view;
  if (view.opaque) return view.typed;
  if (view.typed !== undefined && view.at === view.typed.at * view.stride) return view.typed;
  if (view.plain !== undefined && view.at === 0) return view.plain;
  if (view.of !== undefined && view.at % view.stride === 0) return $ptr(view.of, view.at / view.stride);
  throw new Error("rust-js does not support bytes read as another type's, which reads memory by where it is");
}
