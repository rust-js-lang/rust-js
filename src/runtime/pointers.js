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
  if (pointer === null || !Array.isArray(pointer.of) || pointer.addr !== undefined) {
    throw new Error("rust-js does not support stepping a pointer that isn't into an array, which reads memory by where it is");
  }
  return $ptr(pointer.of, pointer.at + count);
}

// `p.offset_from(q)`: how many items `p` is past `q`, along one array. Of
// two, which Rust leaves undefined, rust-js's error.
function $ptrOffsetFrom(p, q) {
  if (p === q) return 0;
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
function $allocationOf(pointer) {
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
  pointer = $placed(pointer);
  const allocation = $allocationOf(pointer);
  const items = Array.isArray(allocation) && pointer.of !== undefined && typeof pointer.at === "number";
  const range = $addresses(allocation, items ? allocation.length * size : 0);
  if (items && range.stride === undefined) range.stride = size;
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
    return a === b || ($allocationOf(a) === $allocationOf(b) && a.at === b.at && a.offset === b.offset);
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
  const [items, start] = $ptrItems(pointer, count);
  return start === 0 && count === items.length ? items : $slots(items, start, count);
}

// `slice::from_raw_parts_mut(p, n)`: a view of the `n` items from where `p`
// points (ADR 0335), or all of an array, the array itself.
function $ptrSliceMut(pointer, count) {
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

// A pointer as one to bytes, `p.cast::<u8>()`, which JS has none of (ADR
// 0417): one for each, given to the allocator and cast back, `$ptrTyped`,
// but not read, written or stepped through.
const $bytesViews = new WeakMap();

function $ptrBytes(pointer) {
  if (pointer === null || pointer.bytes !== undefined) return pointer;
  let view = $bytesViews.get(pointer);
  if (view === undefined) {
    const refused = () => {
      throw new Error("rust-js does not support reading or writing bytes, which JS has none of, through a pointer to another type");
    };
    view = {
      bytes: pointer,
      get value() {
        return refused();
      },
      set value(value) {
        refused();
      },
    };
    $bytesViews.set(pointer, view);
  }
  return view;
}

// A pointer to bytes as one to a type: what was cast to bytes, or an
// allocation's first item. Of bytes that are bytes, rust-js's error.
function $ptrTyped(pointer) {
  if (pointer === null) return null;
  if (pointer.bytes !== undefined) return pointer.bytes;
  throw new Error("rust-js does not support bytes read as another type's, which reads memory by where it is");
}
