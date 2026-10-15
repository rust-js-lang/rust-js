// A raw pointer is a handle on its place, `{ value }`, one for each place
// (ADR 0407): `object[key]`'s, so two pointers to it are `===`, and `*p` is
// `p.value`. `of` and `at` say where it is, which `add` steps along an array.
const $places = new WeakMap();

function $ptr(object, key) {
  let handles = $places.get(object);
  if (handles === undefined) $places.set(object, (handles = new Map()));
  let handle = handles.get(key);
  if (handle === undefined) {
    handle = {
      of: object,
      at: key,
      get value() {
        return object[key];
      },
      set value(value) {
        object[key] = value;
      },
    };
    handles.set(key, handle);
  }
  return handle;
}

// A pointer to an object a `&mut` is, whose place JS doesn't name: one for
// the object, whose `*p = v` makes it `v` in place, as `*r = v` does.
const $objects = new WeakMap();

function $ptrTo(object) {
  let handle = $objects.get(object);
  if (handle === undefined) {
    handle = {
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
  if (pointer === null || !Array.isArray(pointer.of)) {
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

// `{:?}` of a pointer, its address: `0x0` of `null`; another's, which JS
// hasn't, is rust-js's error where it's shown.
function $ptrShown(pointer) {
  if (pointer === null) return "0x0";
  throw new Error("rust-js does not support showing a pointer's address, which JS hasn't");
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
  if (src.of === dst.of && copy === undefined) {
    src.of.copyWithin(dst.at, src.at, src.at + count);
    return;
  }
  const items = src.of.slice(src.at, src.at + count);
  for (let i = 0; i < count; i++) dst.of[dst.at + i] = copy === undefined ? items[i] : copy(items[i]);
}

// `slice::from_raw_parts(p, n)`: the `n` items from where `p` points, a
// copy, as a shared subslice is, or all of an array, the array itself.
function $ptrSlice(pointer, count) {
  const [items, start] = $ptrItems(pointer, count);
  return start === 0 && count === items.length ? items : items.slice(start, start + count);
}

// `slice::from_raw_parts_mut(p, n)`: a view of the `n` items from where `p`
// points (ADR 0335), or all of an array, the array itself.
function $ptrSliceMut(pointer, count) {
  const [items, start] = $ptrItems(pointer, count);
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
// at the same place depends on the type: arrayvec's `ptr.cast::<u8>()` of
// a type with no size, which reads memory by where it is.
function $ptrRecast(pointer) {
  throw new Error("rust-js does not support a pointer read as another type's, which reads memory by where it is");
}
