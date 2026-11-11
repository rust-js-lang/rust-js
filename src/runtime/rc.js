// A counted `Rc` or `Arc` (ADR 0320): `{ value, strong, weak }`, which each
// clone, and each `Weak`, shares, counted as Rust counts them. Of a
// `#[rust_js::class]`, the instance itself, which JS calls its methods on,
// its box hidden beside it, `inst[$countedKey]`, whose `value` is the instance;
// its `Weak`s are that box, tied to its allocation, so one of an instance
// given back and counted anew never upgrades (#1228).
const $countedKey = Symbol("rust-js.rc");

// The box that counts an `Rc`: itself, or a class instance's hidden one.
function $countedBox(rc) {
  return rc[$countedKey] ?? rc;
}

// What an `Rc` of a type parameter points at, which a counted class may be:
// the instance itself, or a box's `value` (#1228).
function $countedValue(rc) {
  return $countedKey in rc ? rc : rc.value;
}

// `Rc::new(instance)` of a class: the instance, its box hidden beside it,
// one `Rc` and no `Weak`; a box of its own where it was counted before and
// given back, which the old one's `Weak`s don't reach.
function $countedInstance(instance) {
  Object.defineProperty(instance, $countedKey, {
    value: { value: instance, strong: 1, weak: 0 },
    writable: true,
    configurable: true,
  });
  return instance;
}

// The `Rc` a box counts: the class instance it's hidden beside, or itself.
function $countedRc(box) {
  return box.value?.[$countedKey] === box ? box.value : box;
}

// A new `Rc` of `value` held as `rc` is: a class's instance or a box.
function $countedLike(rc, value) {
  return $countedKey in rc ? $countedInstance(value) : { value, strong: 1, weak: 0 };
}

function $rcClone(rc) {
  $countedBox(rc).strong++;
  return rc;
}

// One `Rc` fewer: the last drops what it points at, with `drop` if that has
// a destructor, and its `Weak`s can't upgrade.
function $rcDrop(rc, drop) {
  const box = $countedBox(rc);
  if (--box.strong === 0) {
    // Of an `Rc<dyn T>`, which is its pair, `{ value, impl }` (ADR 0520).
    const value = "impl" in box ? { value: box.value, impl: box.impl } : box.value;
    box.value = undefined;
    drop?.(value);
  }
}

function $downgrade(rc) {
  const box = $countedBox(rc);
  box.weak++;
  return box;
}

function $upgrade(weak) {
  if (weak.strong === 0) return undefined;
  weak.strong++;
  return $countedRc(weak);
}

function $weakClone(weak) {
  weak.weak++;
  return weak;
}

function $weakDrop(weak) {
  weak.weak--;
}

// `Weak::new()` of a counted `Rc`'s: one to nothing, which never upgrades,
// the one each is, so two are `Weak::ptr_eq`, as std's are (ADR 0843).
const $weakNone = { strong: 0, weak: 0 };

// A `Weak`'s `weak_count()`: none once nothing strong is left.
function $weakCount(weak) {
  return weak.strong === 0 ? 0 : weak.weak;
}

// What the last `Rc` points at, taken: its `Weak`s can't upgrade.
function $rcTake(rc) {
  const box = $countedBox(rc);
  const value = box.value;
  box.strong = 0;
  box.value = undefined;
  return value;
}

// `Rc::try_unwrap(rc)`: `Ok` of what it points at if it's the last, else
// `Err` of itself.
function $tryUnwrap(rc) {
  if ($countedBox(rc).strong !== 1) return { TAG: "Err", _0: rc };
  return { TAG: "Ok", _0: $rcTake(rc) };
}

// `Rc::into_inner(rc)`: `Some` of what it points at if it's the last, else
// `None`, and one `Rc` fewer.
function $intoInner(rc) {
  const box = $countedBox(rc);
  if (box.strong !== 1) {
    box.strong--;
    return undefined;
  }
  return $some($rcTake(rc));
}

// `Rc::unwrap_or_clone(rc)`: what it points at if it's the last, else a
// clone of it, `clone`'s where that's more than the value itself, made
// while it's still shared, and then the `Rc` dropped, `drop` what the last
// drops, as a panic unwinds too.
function $unwrapOrClone(rc, clone, drop) {
  const box = $countedBox(rc);
  if (box.strong === 1) return $rcTake(rc);
  try {
    return clone ? clone(box.value) : box.value;
  } finally {
    $rcDrop(rc, drop);
  }
}

// `Rc::make_mut(&mut rc)`: the `Rc` to change, itself if it's the only one
// and nothing's weak to it, else a new one of what it pointed at, moved if
// it was the last strong one, else cloned while it's still shared, and then
// the old one dropped, `drop` what the last drops. A clone that panics
// leaves it as it was.
function $makeMut(rc, clone, drop) {
  const box = $countedBox(rc);
  if (box.strong === 1) {
    if (box.weak === 0) return rc;
    return $countedLike(rc, $rcTake(rc));
  }
  const made = $countedLike(rc, clone ? clone(box.value) : box.value);
  $rcDrop(rc, drop);
  return made;
}

// `Rc::new_cyclic(f)`: `f` given a `Weak` to what it makes, which can't
// upgrade until it's made; of a class, `instance`, the instance it makes,
// its box hidden beside it (#1228).
function $newCyclic(f, instance) {
  const rc = { value: undefined, strong: 0, weak: 1 };
  rc.value = f(rc);
  rc.strong = 1;
  rc.weak--;
  if (!instance) return rc;
  Object.defineProperty(rc.value, $countedKey, { value: rc, writable: true, configurable: true });
  return rc.value;
}

// A counted `Rc<dyn Sub>` as an `Rc<dyn Super>` (#186): its pair of the
// supertrait's dictionary, `dictionary`, whose value and counts are the
// `Rc`'s own, shared, as Rust's upcast shares its allocation. One for each
// supertrait, `key`, so two of the same `Rc` are the same box, as
// `Rc::ptr_eq` finds them.
function $rcUpcast(rc, key, dictionary) {
  rc.$views ??= new Map();
  let view = rc.$views.get(key);
  if (view === undefined) {
    view = {
      impl: dictionary,
      get value() {
        return rc.value;
      },
      set value(value) {
        rc.value = value;
      },
      get strong() {
        return rc.strong;
      },
      set strong(strong) {
        rc.strong = strong;
      },
      get weak() {
        return rc.weak;
      },
      set weak(weak) {
        rc.weak = weak;
      },
    };
    rc.$views.set(key, view);
  }
  return view;
}
