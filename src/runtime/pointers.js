// A raw pointer is a handle on its place, `{ value }`, one for each place
// (ADR 0407): `object[key]`'s, so two pointers to it are `===`, and `*p` is
// `p.value`. `of` and `at` say where it is, which `add` steps along an array;
// `offset`, a field's byte offset in its object, says where its address is
// (ADR 0562); an item's is its index times its size.
const $places = new WeakMap();
// Where each object a record names is stored by value (#1113): its
// container, key, offset there, an array's stride, and `within`, where in
// the slot it is past what holds it (#1114). Written where a reference or a
// pointer is made to it, `$stored`, `$storedItems`, `$storedValue`.
const $items = new WeakMap();

// `root` and `at`, of a place reached through more than one field or item:
// the object its projections start from, and the place's byte offset in it,
// where its address is (#1089).
function $ptr(object, key, offset, root, at) {
  // An item of a part of a slice, `&v[1..3]`'s: the item of the array it's
  // of, where Rust's is (#1185); of a view of another type's bytes, or of
  // one place, its first item's pointer stepped to it (#1257, #1280).
  const part = typeof key === "number" ? object[$placeOfPart] : undefined;
  if (part?.first !== undefined) return $ptrAdd(part.first, key);
  if (part !== undefined) return $ptr(part.of, part.start + key, offset, root, at);
  // What a cell field that's the property it holds lends, through a
  // handle on it, or its one cell, `$fieldCell(x, "f")` (#1114).
  const field = key === "value" ? $cellFields.get(object) : undefined;
  if (offset === undefined || field !== undefined) {
    const at = field ?? { object, key };
    const content = $fieldContents.get(at.object)?.get(at.key);
    if (content !== undefined) return $ptrIn(at.object, at.key, content.offset, content.within);
  }
  let handles = $places.get(object);
  if (handles === undefined) $places.set(object, (handles = new Map()));
  let handle = handles.get(key);
  if (handle !== undefined && root !== undefined && (handle.root === undefined || !$rooted(handle))) {
    handle.root = root;
    handle.rootAt = at;
  }
  if (handle === undefined) {
    handle = {
      of: object,
      at: key,
      offset,
      root,
      rootAt: at,
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
    throw $unsupported("rust-js does not support a pointer to a value JS holds as itself, a copy, which reads memory by where it is");
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
// pointer is into; of another, where JS has no place, and of a tuple's
// field, whose neighbour is where its layout puts it (#1081), rust-js's
// error.
function $ptrAdd(pointer, count) {
  if (pointer !== null && pointer.bytes !== undefined) {
    return $byteView({ ...pointer, at: pointer.at + count * pointer.width });
  }
  if (pointer === null || !Array.isArray(pointer.of) || pointer.addr !== undefined || pointer.offset !== undefined) {
    // Not stepped, where it is (#1280).
    if (pointer !== null && count === 0) return pointer;
    throw $unsupported("rust-js does not support stepping a pointer that isn't into an array, which reads memory by where it is");
  }
  return $ptr(pointer.of, pointer.at + count);
}

// `p.offset_from(q)`: how many items `p` is past `q`, along one array. Of
// two, which Rust leaves undefined, rust-js's error. Of a view of an
// array's bytes and a pointer to its items, a pointer stepped in bytes and
// cast back, `p.cast::<u8>().add(8).cast::<u64>()`, how many of the view's
// width its bytes are past the other's, as both are into one array (#1254).
function $ptrOffsetFrom(p, q) {
  if (p === q) return 0;
  if (p?.bytes !== undefined || q?.bytes !== undefined) {
    const [from, to] = [$byteOffset(p), $byteOffset(q)];
    if (from !== undefined && to !== undefined && from.of === to.of) {
      return (from.at - to.at) / (p.bytes !== undefined ? p.width : q.width);
    }
  }
  if (p.of !== q.of || !Array.isArray(p.of) || p.offset !== undefined || q.offset !== undefined) {
    throw $unsupported("rust-js does not support the distance between pointers to two places, which reads memory by where it is");
  }
  return p.at - q.at;
}

// Where a pointer is in the bytes of what it's into: a view's own offset,
// or an array item's, at its index times the stride a view of the array
// read it by; none of another.
function $byteOffset(pointer) {
  if (pointer?.bytes !== undefined) return { of: $bytesOf(pointer), at: pointer.at };
  if (!Array.isArray(pointer?.of) || typeof pointer.at !== "number" || pointer.offset !== undefined) return undefined;
  const stride = $strides.get(pointer.of);
  return stride === undefined ? undefined : { of: pointer.of, at: pointer.at * stride };
}

// `ptr::replace(p, v)`: what it pointed to, and `v` there now.
function $ptrReplace(pointer, value) {
  const old = pointer.value;
  pointer.value = value;
  return old;
}

// `ptr::swap(a, b)`: what each points to, the other's; and
// `ptr::swap_nonoverlapping(a, b, count)`, of each of the `count` items
// from where each points, along its array or its bytes' view.
function $ptrSwap(a, b, count = 1) {
  for (let i = 0; i < count; i++) {
    const [x, y] = i === 0 ? [a, b] : [$ptrAdd(a, i), $ptrAdd(b, i)];
    const held = x.value;
    x.value = y.value;
    y.value = held;
  }
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
// `$ptrTo` of an object that's an array's item or a tuple's field, that
// item's or field's.
const $strides = new WeakMap();

function $allocationOf(pointer) {
  if (pointer.bytes !== undefined) return pointer.plain !== undefined ? $allocationOf(pointer.plain) : $bytesOf(pointer);
  return $placed(pointer).of ?? pointer;
}

function $placed(pointer) {
  // A reference that's an array's item itself, the object, `transmute(&held.0)`
  // of a tuple struct's field: the item's place (#1019); of an object
  // `within` its slot, the object's.
  const own = $itemOf(pointer);
  if (own !== undefined) return own.within ? $ptrTo(pointer) : $ptr(own.of, own.at, own.offset);
  // An object's handle, or one on what holds it as far out as its slot,
  // `$ptrOuter`'s: the slot's (#1114).
  if (pointer.at === undefined && pointer.of !== undefined) {
    const item = $itemOf(pointer.of);
    if (item !== undefined && (item.within ?? 0) === (pointer.outer ?? 0)) return $ptr(item.of, item.at, item.offset);
  }
  return pointer;
}

// A handle's byte offset in what it's on: a `{ value }` cell's content's,
// where std lent it, its own (#1114); and what's `within` its slot further.
function $handleOffset(pointer) {
  const offset = pointer.at === "value" && $contents.has(pointer.of) ? $contents.get(pointer.of) : pointer.offset;
  return typeof offset === "number" ? offset + (pointer.within ?? 0) : offset;
}

// Where `object` is stored by value, `$items`' record of it, where it's
// there still; none of one moved since, which is its own allocation till a
// borrow of where it is now records it again, as Rust's is where it was
// moved to (#1113).
function $itemOf(object) {
  const item = $items.get(object);
  if (item === undefined) return undefined;
  if ($slot(item.of, item.at) === object) return item;
  $items.delete(object);
  return undefined;
}

// Where the object a field's or an item's handle is on is stored, recorded
// where a reference or a pointer is made into it, so its address is in its
// container's allocation, whichever pointer asks first (#1113).
// `within`: where in the slot the object is, of a slot whose value JS holds
// as the object it holds, an `Option`'s payload past its tag, a `RefCell`'s
// value past its borrow flag (#1114).
function $stored(handle, stride, within) {
  const value = $slot(handle.of, handle.at);
  if (value !== null && typeof value === "object") {
    $items.set(value, { of: handle.of, at: handle.at, offset: handle.offset, stride, within });
  }
}

// Where a `{ value }` cell, a `RefCell`'s, a `Cell`'s, a lock's, holds what
// it lends, `offset` bytes into it, recorded where std lends it (#1114): a
// handle on its `value` is there, and an object it holds, `within` bytes
// further into the slot, as `$stored`'s is.
const $contents = new WeakMap();

// Recorded as std's call is given it, `$borrow($storedValue(cell, 4))`:
// the cell. `within` `null`: what it holds is an allocation of its own, a
// `Vec`'s items.
function $storedValue(cell, offset, within) {
  $contents.set(cell, offset);
  const value = cell.value;
  if (within !== null && value !== null && typeof value === "object") {
    $items.set(value, { of: cell, at: "value", offset, stride: undefined, within });
  }
  return cell;
}

// Where a cell field that's the property it holds, a `RefCell`'s of a
// number (ADR 0863, amended), keeps that: `offset`, the field's in its
// struct, `within`, past its borrow flag (#1114). A handle on the property
// made of what the cell lends, which says no offset, is one on that.
const $fieldContents = new WeakMap();

function $storedIn(object, key, offset, within) {
  let fields = $fieldContents.get(object);
  if (fields === undefined) $fieldContents.set(object, (fields = new Map()));
  fields.set(key, { offset, within });
}

// A pointer to what JS holds as the object it holds, `within` bytes before
// it: an `Option` of an object whose payload is past its tag, a `RefCell`
// of an object, which is the object (#1114). One for each object and
// offset; it reads and writes the object, as `$ptrTo`'s does, and its
// address is the object's less `within`.
const $outers = new WeakMap();

function $ptrOuter(object, within) {
  if (object === null || typeof object !== "object") {
    throw $unsupported("rust-js does not support a pointer to an `Option` that holds nothing, reached by a reference or a box, which JS holds as `undefined`, no place (#1132): point at it where it's stored, `&raw const o` of the variable or field");
  }
  if (within === 0) return $ptrTo(object);
  let handles = $outers.get(object);
  if (handles === undefined) $outers.set(object, (handles = new Map()));
  let handle = handles.get(within);
  if (handle === undefined) {
    handle = {
      of: object,
      at: undefined,
      offset: 0,
      outer: within,
      get value() {
        return object;
      },
      set value(value) {
        $assign(object, value);
      },
    };
    handles.set(within, handle);
  }
  return handle;
}

// A pointer to what a slot holds `within` bytes into it, of a slot whose
// value JS holds as what it holds, a number: a `RefCell` field's value, an
// `Option`'s payload (#1114). A place of its own, apart from the slot's,
// which reads and writes the slot.
const $inners = new WeakMap();

function $ptrIn(object, key, offset, within) {
  let handles = $inners.get(object);
  if (handles === undefined) $inners.set(object, (handles = new Map()));
  const id = `${typeof key}:${key}@${within}`;
  let handle = handles.get(id);
  if (handle === undefined) {
    handle = {
      of: object,
      at: key,
      offset,
      within,
      get value() {
        return $slot(object, key);
      },
      set value(value) {
        object[key] = value;
      },
    };
    handles.set(id, handle);
  }
  return handle;
}

// Whether a rooted handle's object is in its root still: one of the
// containers its records say it's stored in, outward, is the root, as it
// was where the handle was rooted, each step of its place recorded there,
// the root perhaps stored in another; or none is, moved since to another
// container, whose records say so (#1113).
function $rooted(handle) {
  let object = handle.of;
  while (object !== handle.root) {
    const item = $itemOf(object);
    if (item === undefined) return false;
    object = item.of;
  }
  return true;
}

// Each object a `Vec`'s or an array's items are, recorded as `$stored`'s
// are, at its index then, found again where the array moves it (#1113).
function $storedItems(items, stride, within) {
  for (let i = 0; i < items.length; i++) {
    const value = items[i];
    if (value !== null && typeof value === "object") $items.set(value, { of: items, at: i, offset: undefined, stride, within });
  }
}

// The bytes past each allocation's own kept free, the room before the next
// one's base where what holds an object no slot is recorded as holding,
// `$ptrOuter`'s, is (#1114).
const $outerRoom = 64;

function $addresses(allocation, bytes) {
  let range = $ranges.get(allocation);
  if (range === undefined) {
    const length = Math.max(4096, Math.ceil((bytes + $outerRoom) / 4096) * 4096);
    if ($nextBase + length > 0x1_0000_0000) {
      throw $unsupported("rust-js does not support more addresses than wasm32 has: the address space is used up");
    }
    range = { base: $nextBase, end: $nextBase + length, of: allocation, stride: undefined };
    $nextBase += length;
    $ranges.set(allocation, range);
    $exposed.push(range);
  }
  return range;
}

// An empty array at `addr`, a slice of no items made of a pointer with no
// place, `from_raw_parts(NonNull::dangling(), 0)`: its pointer is at the
// address it was made of, as Rust's is (ADR 0562). Any other empty array
// is an allocation of no bytes, dangling as a value of no size is: at its
// item's size, a primitive's alignment, which Rust's `[]` is at.
function $emptyAt(addr, empty = []) {
  $ranges.set(empty, { base: addr, end: addr, of: empty, stride: undefined });
  return empty;
}

// A handle's byte offset in its allocation: a field's, its own, a tuple's
// too, which JS holds as an array (#1081); an array's item's, its index
// times the range's stride. `null`, a field's of a type its caller lays
// out.
function $offsetIn(pointer, range) {
  if (pointer.bytes !== undefined) {
    return pointer.at + (pointer.plain !== undefined ? $offsetIn(pointer.plain, range) : 0);
  }
  if (pointer.of === undefined || pointer.at === undefined) return 0;
  const offset = $handleOffset(pointer);
  if (typeof offset === "number") return offset;
  // An array's first item is at its start, whatever its stride.
  if (offset === undefined && Array.isArray(pointer.of) && typeof pointer.at === "number") {
    return (pointer.at === 0 ? 0 : pointer.at * range.stride) + (pointer.within ?? 0);
  }
  throw $unsupported("rust-js does not support the address of a field of a type its caller lays out, which reads memory by where it is");
}

// `inner`: asked on the way to another place's, a container's or a root's,
// which exposes only the place the program asked of, so an address found
// again is its (#1114).
function $addr(pointer, size, inner) {
  if (pointer === null) return 0;
  if (typeof pointer !== "object" && typeof pointer !== "function") {
    throw $unsupported("rust-js does not support the address of a value JS holds as itself, a copy, which reads memory by where it is");
  }
  if (pointer.addr !== undefined) return pointer.addr;
  if (pointer.bytes !== undefined && pointer.opaque) return $addr(pointer.typed, size, inner);
  // A view of the bytes of an array stored by value in another object, a
  // struct's field's: at its offset past the array's first item, which is
  // where the record puts it, as its items' pointers are, not an
  // allocation of the array's own.
  if (pointer.bytes !== undefined && pointer.plain === undefined && $itemOf(pointer.of) !== undefined) {
    return $addr($ptr(pointer.of, 0), pointer.stride, true) + pointer.at;
  }
  pointer = $placed(pointer);
  // A pointer to what holds an object no slot is recorded as holding it at
  // `within`: the object's address less it, in the room before each
  // allocation's base that `$addresses` keeps (#1114).
  if (pointer.outer !== undefined) {
    if (pointer.outer > $outerRoom) {
      throw $unsupported("rust-js does not support the address of what holds an object this far before it, which reads memory by where it is");
    }
    const addr = $addr($ptrTo(pointer.of), 0, true) - pointer.outer;
    if (!inner) $exposedPlaces.set(addr, pointer);
    return addr;
  }
  // A place reached through more than one field or item: at its offset in
  // the root its projections start from, whose address is the root's
  // allocation's (#1089).
  // Not where the object it's on was moved out of its root since (#1113).
  if (pointer.root !== undefined && pointer.bytes === undefined && $rooted(pointer)) {
    const addr = $addr($ptrTo(pointer.root), 0, true) + pointer.rootAt;
    if (!inner) $exposedPlaces.set(addr, pointer);
    return addr;
  }
  // A field of an object stored in another, as its records say, an
  // array's item, a tuple's or a struct's field, a cell's value: in the
  // allocation that holds it, at the object's address and the field's
  // offset (#1019, #1081, #1113), the object `within` its slot past what
  // holds it (#1114). An array's item recorded with no size, generic
  // code's, is its own allocation.
  const item = pointer.bytes === undefined && pointer.of !== undefined ? $itemOf(pointer.of) : undefined;
  // Where in the object recorded it is: a field's offset, an item's index
  // times its size.
  const offset =
    item === undefined
      ? undefined
      : typeof pointer.offset === "number"
        ? $handleOffset(pointer)
        : pointer.offset === undefined && Array.isArray(pointer.of) && typeof pointer.at === "number"
          ? (pointer.at === 0 ? 0 : pointer.at * size) + (pointer.within ?? 0)
          : undefined;
  if (offset !== undefined) {
    // An array's item recorded with its size: the array's address and its
    // index times that (#1113).
    const start =
      typeof item.offset === "number"
        ? $addr($ptr(item.of, item.at, item.offset), 0, true)
        : typeof item.stride === "number"
          ? $addr($ptrTo(item.of), 0, true) + item.at * item.stride
          : undefined;
    if (start !== undefined) {
      const addr = start + (item.within ?? 0) + offset;
      if (!inner) $exposedPlaces.set(addr, pointer);
      return addr;
    }
  }
  const allocation = $allocationOf(pointer);
  const view = pointer.bytes !== undefined && pointer.plain === undefined;
  // A tuple's field, which JS holds as an array's item, is at its offset
  // (#1081), no item.
  const items =
    Array.isArray(allocation) &&
    ((pointer.of !== undefined && typeof pointer.at === "number" && pointer.offset === undefined) || view);
  const stride = view ? pointer.stride : size;
  if (items && allocation.length === 0 && !$ranges.has(allocation)) $emptyAt(Math.max(stride, 1), allocation);
  const range = $addresses(allocation, items ? allocation.length * stride : 0);
  if (items && range.stride === undefined) range.stride = stride;
  const addr = range.base + $offsetIn(pointer, range);
  if (!items && !inner) $exposedPlaces.set(addr, pointer);
  return addr;
}

// A pointer with no place: an address, and the provenance of the pointer
// it was made from, if any, which `$ptrAt` may bring it back to. Of a value
// of no size, `size` 0, which Rust reads and writes through any pointer
// that isn't null: reads give `zero`, the JS value of one, `undefined` or
// `[]`, and writes change nothing.
function $ptrNone(addr, provenance, size, zero) {
  return {
    addr,
    provenance,
    get value() {
      if (size === 0) return zero;
      throw $unsupported(`rust-js does not support reading through a pointer to no place, at 0x${addr.toString(16)}, which reads memory by where it is`);
    },
    set value(value) {
      if (size === 0) return;
      throw $unsupported(`rust-js does not support writing through a pointer to no place, at 0x${addr.toString(16)}, which reads memory by where it is`);
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
function $ptrFrom(addr, size, zero) {
  addr >>>= 0;
  if (addr === 0) return null;
  const range = $addressesAt(addr);
  const provenance = range?.of === undefined ? null : $ptrTo(range.of);
  return (range && $placeAt(addr, range, size)) ?? $ptrNone(addr, provenance, size, zero);
}

// `p.with_addr(n)`, `map_addr`, `wrapping_add` and `wrapping_byte_add`
// (ADR 0562): a pointer with `p`'s provenance at `n`, a 32-bit address as
// wasm32's `usize` wraps: the place there, where its allocation has one,
// else a pointer with no place that remembers it.
function $ptrAt(pointer, addr, size, zero) {
  addr >>>= 0;
  if (pointer === null) return $ptrFrom(addr, size, zero);
  const provenance = pointer.addr === undefined ? pointer : pointer.provenance;
  if (provenance === null) return addr === 0 ? null : $ptrNone(addr, null, size, zero);
  if (pointer.addr === undefined && addr === $addr(pointer, size)) return pointer;
  const allocation = $allocationOf(provenance);
  // An item of an array stored in another, a cell's value say, which has no
  // addresses of its own, where its record puts it (#1114).
  if (Array.isArray(allocation) && size > 0 && $itemOf(allocation) !== undefined) {
    const at = addr - $addr($ptrTo(allocation), 0, true);
    if (at >= 0 && at % size === 0 && at / size <= allocation.length) return $ptr(allocation, at / size);
  }
  const range = $ranges.get(allocation);
  if (range !== undefined && range.base <= addr && addr < range.end) {
    const place = $placeAt(addr, range, size);
    if (place !== undefined) return place;
  }
  return addr === 0 ? null : $ptrNone(addr, provenance, size, zero);
}

// `p == q` (ADR 0562): the same place, or the same address; `null`'s is 0.
function $ptrEq(p, q, size) {
  if (p === q) return true;
  if (p === null || q === null) return false;
  if (p.addr !== undefined && q.addr !== undefined) return p.addr === q.addr;
  if (p.addr === undefined && q.addr === undefined) {
    const [a, b] = [$placed(p), $placed(q)];
    if (a === b) return true;
    // An object's pointer and its slot's, where nothing recorded it there, a
    // crate that asks no address: the slot holds it, one place (#1019,
    // #1114).
    const holds = (object, slot) =>
      object.at === undefined && object.outer === undefined && slot.at !== undefined && slot.within === undefined && slot.bytes === undefined && $slot(slot.of, slot.at) === object.of;
    if (holds(a, b) || holds(b, a)) return true;
    // Of two allocations, by address where each has one, or is empty: two
    // empty arrays dangling at one address are equal, as Rust's are.
    if ($allocationOf(a) !== $allocationOf(b)) {
      const known = (x) => $ranges.has($allocationOf(x)) || (Array.isArray(x.of) && x.of.length === 0);
      return known(a) && known(b) && $addr(a, size) === $addr(b, size);
    }
    // Two of one kind, by where they are; a byte view and a handle, by the
    // offset both are at, where the array's stride is known.
    if ((a.bytes !== undefined) === (b.bytes !== undefined)) {
      return a.at === b.at && a.offset === b.offset && (a.within ?? 0) === (b.within ?? 0) && (a.outer ?? 0) === (b.outer ?? 0);
    }
    const [bytes, handle] = a.bytes !== undefined ? [a, b] : [b, a];
    const stride = $strides.get($allocationOf(handle));
    return stride !== undefined && typeof handle.at === "number" && handle.at * stride === bytes.at;
  }
  const [handle, addr] = p.addr === undefined ? [$placed(p), q.addr] : [$placed(q), p.addr];
  const range = $ranges.get($allocationOf(handle));
  // An empty array not yet addressed is at its items' dangling address, as
  // `$addr` puts it: `Vec::new().as_ptr() == NonNull::dangling().as_ptr()`.
  // One in an allocation stored in another, which has no addresses of its
  // own, by where its record puts it (#1114).
  if (range === undefined && $itemOf($allocationOf(handle)) !== undefined) return $addr(handle, size, true) === addr;
  if (range === undefined) return Array.isArray(handle.of) && handle.of.length === 0 && $addr(handle, size) === addr;
  return range.base + $offsetIn(handle, range) === addr;
}

// `ptr::eq(a, b)` of two slices, `==` of two `*const [T]`: the same length,
// and the same place for their first item, a part's in the array it's of
// (#1185), as Rust compares their addresses and lengths. An empty one's is
// where it is, `v[2..2]` at `v`'s third.
function $slicePtrEq(a, b, size) {
  // A slice's pointer is the slice, or a handle on it, `$ptrTo(items)`, of
  // one read out of where it's held.
  const items = (p) => (p !== null && !Array.isArray(p) && p.at === undefined && Array.isArray(p.of) ? p.of : p);
  [a, b] = [items(a), items(b)];
  return a === b || (a !== null && b !== null && a.length === b.length && $ptrEq($ptr(a, 0), $ptr(b, 0), size));
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
    throw $unsupported("rust-js does not support copying items but between arrays, which reads memory by where it is");
  }
  if (src.of === dst.of && copy === undefined && Math.max(src.at, dst.at) + count <= src.of.length) {
    src.of.copyWithin(dst.at, src.at, src.at + count);
    return;
  }
  const items = $slots(src.of, src.at, count);
  for (let i = 0; i < count; i++) dst.of[dst.at + i] = copy === undefined ? items[i] : copy(items[i]);
}

// `ptr::write_bytes(dst, byte, n)`: each of the `count` bytes from where
// `bytes`, `dst` seen as bytes, points `byte`, an item's bytes as wasm32
// lays them out (#1133).
function $ptrFill(bytes, byte, count) {
  if (count === 0) return;
  const view = $asBytes(bytes);
  for (let i = 0; i < count; i++) $byteSet(view, i, byte);
}

// `slice::from_raw_parts(p, n)`: the `n` items from where `p` points, a
// copy, as a shared subslice is, or all of an array, the array itself. Of
// a pointer to bytes, each item read as it's typed, `width` bytes apart: a
// `*const u32`'s words, not its bytes (#1107).
function $ptrSlice(pointer, count) {
  if (pointer.bytes !== undefined) {
    return Array.from({ length: count }, (_, i) => $byteView({ ...pointer, at: pointer.at + i * pointer.width }).value);
  }
  const [items, start] = $ptrItems(pointer, count);
  return start === 0 && count === items.length ? items : $slots(items, start, count);
}

// `slice::from_raw_parts_mut(p, n)`: a view of the `n` items from where `p`
// points (ADR 0335), or all of an array, the array itself.
function $ptrSliceMut(pointer, count) {
  if (pointer.bytes !== undefined) {
    // A byte array's own bytes, the array's items.
    if ($itemsAreBytes(pointer) && pointer.width === 1 && pointer.signed === false) {
      $allocUsed(pointer, "bytes");
      return $view(pointer.of, pointer.at, pointer.at + count);
    }
    // Another's, a view of its bytes `width` at a time, each item read and
    // written where its bytes are, through their codec, or a `[u8; N]`'s
    // that writes through (#1257); a pointer into it is the view stepped to
    // that item.
    const target = [];
    target[$placeOfPart] = { first: pointer };
    return $proxy(
      count,
      (i) => $ptrAdd(pointer, i).value,
      (i, item) => {
        $ptrAdd(pointer, i).value = item;
      },
      target,
    );
  }
  // One place that isn't an array's item, a local's or a field's: a view
  // of it, written through its handle (#1280).
  if (count === 1 && !Array.isArray(pointer.of)) {
    const target = [];
    target[$placeOfPart] = { first: pointer };
    return $proxy(
      1,
      () => pointer.value,
      (_, item) => {
        pointer.value = item;
      },
      target,
    );
  }
  const [items, start] = $ptrItems(pointer, count);
  if (start + count > items.length && $spares.has(items)) {
    throw $unsupported("rust-js does not support a mutable slice past a vector's length, over what's left there, which reads memory by where it is");
  }
  return start === 0 && count === items.length ? items : $view(items, start, start + count);
}

// The array a pointer is into, and its index there, for `count` items from
// it: none, of no array but one place, is an array of it.
function $ptrItems(pointer, count) {
  if (Array.isArray(pointer.of)) return [pointer.of, pointer.at];
  if (count === 0 && pointer.addr !== undefined) return [$emptyAt(pointer.addr), 0];
  if (count <= 1) return [count === 0 ? [] : [pointer.value], 0];
  throw $unsupported("rust-js does not support a slice of items that aren't an array's, which reads memory by where it is");
}

// A pointer to an array's item, as one to an array of `length` items from
// it: arrayvec's `ptr::read(p as *const [T; CAP])` of the whole, and a
// slice's part, `fst.as_ptr() as *const [T; N]`, a view of those items,
// placed where they are (#1218). Past the array's end, it would be memory
// by where it is.
function $ptrWhole(pointer, length) {
  if (Array.isArray(pointer.of) && typeof pointer.at === "number" && pointer.at + length <= pointer.of.length) {
    if (pointer.at === 0 && pointer.of.length === length) return $ptrTo(pointer.of);
    return $ptrTo($view(pointer.of, pointer.at, pointer.at + length));
  }
  throw $unsupported("rust-js does not support a pointer to part of an array as one to an array past its end, which reads memory by where it is");
}

// A pointer as one to another type, in generic code where whether JS has it
// at the same place depends on the type, which reads memory by where it is.
function $ptrRecast(pointer) {
  throw $unsupported("rust-js does not support a pointer read as another type's, which reads memory by where it is");
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
    // Each item read and written through its type's codec, a signed
    // integer's, a float's or a struct's, as wasm32 lays it out (#1137).
    return $byteView({ of: pointer.of, at: pointer.at * stride, stride, typed: pointer, encode, decode });
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
        throw $unsupported("rust-js does not support reading or writing bytes, which JS has none of, through a pointer to another type");
      }
      if (this.signed === "bytes") return $bytesRead(this);
      return $readInt(this, this.width, this.signed);
    },
    set(value) {
      if (this.opaque) {
        throw $unsupported("rust-js does not support reading or writing bytes, which JS has none of, through a pointer to another type");
      }
      if (this.signed === "bytes") {
        for (let i = 0; i < this.width; i++) $byteSet(this, i, value[i]);
        return;
      }
      $writeInt(this, this.width, value);
    },
  });
}

// A `[u8; N]` read through a view of another type's bytes: those bytes as
// they are now, a copy, as a read of a `Copy` value is, whose every write,
// `(*p)[1] = 9` or through a `&mut *p`, is also a write of the byte it
// was read from (#1266). What's read and kept is copied where it's changed
// in place after, as any such read is, so only a write through the place
// it was read from reaches these; and a pointer into it is into the bytes.
function $bytesRead(view) {
  const bytes = Array.from({ length: view.width }, (_, i) => $byteAt(view, i));
  const target = [];
  target[$placeOfPart] = { first: $ptrRetyped(view, 1, false) };
  return $proxy(
    view.width,
    (i) => bytes[i],
    (i, byte) => {
      bytes[i] = byte;
      $byteSet(view, i, byte);
    },
    target,
  );
}

// A pointer to `u8`s as the view of their bytes it is: a view already; or
// `$ptr(array, i)` into a byte array, whose items are its bytes, no codec;
// or one `u8`'s handle, the byte its value is.
function $asBytes(pointer) {
  if (pointer === null || pointer.bytes !== undefined) return pointer;
  if (Array.isArray(pointer.of) && typeof pointer.at === "number") {
    $strides.set(pointer.of, 1);
    return $byteView({ of: pointer.of, at: pointer.at, stride: 1, typed: pointer });
  }
  return $ptrBytes(pointer, 1, (value) => [value], (bytes) => bytes[0]);
}

// Whether a view's items are its bytes, a `u8` array's, read and written
// as they are; an `i8` array's, a byte each too, go through their codec,
// as a number of its sign isn't its byte (#1253).
function $itemsAreBytes(view) {
  return view.plain === undefined && view.stride === 1 && view.encode === undefined;
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

// An allocation of `alloc`'s and how it's used, by its slots: as bytes,
// read or written through a byte pointer, or as the items of a type it's
// cast to (ADR 0417). The first use says which; the other after it would be
// one type's memory read as another's (#1099).
const $allocUses = new WeakMap();

function $allocUsed(view, use) {
  if (view.allocated === undefined) return;
  const used = $allocUses.get(view.of);
  if (used === undefined) $allocUses.set(view.of, use);
  else if (used !== use) {
    throw $unsupported("rust-js does not support an allocation's bytes and its items of another type both, which reads memory by where it is");
  }
}

// The bytes of `value`, an item a view's bytes are of, through its codec;
// of one not yet written, a `MaybeUninit`'s, `undefined`, zeros, which no
// read sees before they're written, as reading them is undefined (#1279).
function $encoded(view, value) {
  return value === undefined ? new Array(view.stride).fill(0) : view.encode(value);
}

// The byte `i` past where `view` is.
function $byteAt(view, i) {
  $allocUsed(view, "bytes");
  const at = view.at + i;
  if (view.plain !== undefined) return $encoded(view, view.plain.value)[at];
  if ($itemsAreBytes(view)) return $slot(view.of, at);
  const item = Math.floor(at / view.stride);
  return $encoded(view, $slot(view.of, item))[at % view.stride];
}

function $byteSet(view, i, byte) {
  $allocUsed(view, "bytes");
  const at = view.at + i;
  if (view.plain !== undefined) {
    const bytes = $encoded(view, view.plain.value);
    bytes[at] = byte;
    view.plain.value = view.decode(bytes);
    return;
  }
  if ($itemsAreBytes(view)) {
    view.of[at] = byte;
    return;
  }
  const item = Math.floor(at / view.stride);
  const bytes = $encoded(view, $slot(view.of, item));
  bytes[at % view.stride] = byte;
  view.of[item] = view.decode(bytes);
}

// `read_unaligned` of a `width`-byte integer at a byte pointer: the bytes
// there, little-endian, `signed` as the type is; `write_unaligned` the
// reverse.
function $readInt(view, width, signed) {
  if (width === 1 && $itemsAreBytes(view) && !signed) {
    $allocUsed(view, "bytes");
    return $slot(view.of, view.at);
  }
  const bytes = Array.from({ length: width }, (_, i) => $byteAt(view, i));
  return $fromBytes(bytes, width, true, signed);
}

function $writeInt(view, width, value) {
  // An `i8`'s byte, of its sign, as `u8`'s it is (#1253).
  const bytes = width === 1 ? [value & 0xff] : $toBytes(value, width, true);
  for (let i = 0; i < width; i++) $byteSet(view, i, bytes[i]);
}

// A pointer to bytes as one to a type: what was cast to bytes, or the
// array's item its bytes start at. Of bytes that are bytes alone, rust-js's
// error.
function $ptrTyped(view) {
  if (view === null || view.bytes === undefined) return view;
  if (view.opaque) return view.typed;
  // An allocation's first slot, its items' (ADR 0417); one past it, whose
  // item that is its type's size says, which JS hasn't here.
  if (view.allocated !== undefined) {
    if (view.at !== 0) {
      throw $unsupported("rust-js does not support a pointer into an allocation's bytes as one to another type, which reads memory by where it is");
    }
    $allocUsed(view, "items");
    return view.typed;
  }
  if (view.typed !== undefined && view.at === view.typed.at * view.stride) return view.typed;
  if (view.plain !== undefined && view.at === 0) return view.plain;
  if (view.of !== undefined && view.at % view.stride === 0) return $ptr(view.of, view.at / view.stride);
  throw $unsupported("rust-js does not support bytes read as another type's, which reads memory by where it is");
}
