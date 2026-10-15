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
