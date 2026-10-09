// A counted `Rc` or `Arc` (ADR 0320): `{ value, strong, weak }`, which each
// clone, and each `Weak`, shares, counted as Rust counts them.
function $rcClone(rc) {
  rc.strong++;
  return rc;
}

// One `Rc` fewer: the last drops what it points at, with `drop` if that has
// a destructor, and its `Weak`s can't upgrade.
function $rcDrop(rc, drop) {
  if (--rc.strong === 0) {
    const value = rc.value;
    rc.value = undefined;
    drop?.(value);
  }
}

function $downgrade(rc) {
  rc.weak++;
  return rc;
}

function $upgrade(weak) {
  if (weak.strong === 0) return undefined;
  weak.strong++;
  return weak;
}

function $weakClone(weak) {
  weak.weak++;
  return weak;
}

function $weakDrop(weak) {
  weak.weak--;
}

// A `Weak`'s `weak_count()`: none once nothing strong is left.
function $weakCount(weak) {
  return weak.strong === 0 ? 0 : weak.weak;
}

// What the last `Rc` points at, taken: its `Weak`s can't upgrade.
function $rcTake(rc) {
  const value = rc.value;
  rc.strong = 0;
  rc.value = undefined;
  return value;
}

// `Rc::try_unwrap(rc)`: `Ok` of what it points at if it's the last, else
// `Err` of itself.
function $tryUnwrap(rc) {
  if (rc.strong !== 1) return { TAG: "Err", _0: rc };
  return { TAG: "Ok", _0: $rcTake(rc) };
}

// `Rc::into_inner(rc)`: `Some` of what it points at if it's the last, else
// `None`, and one `Rc` fewer.
function $intoInner(rc) {
  if (rc.strong !== 1) {
    rc.strong--;
    return undefined;
  }
  return $some($rcTake(rc));
}

// `Rc::unwrap_or_clone(rc)`: what it points at if it's the last, else a
// clone of it, `clone`'s where that's more than the value itself.
function $unwrapOrClone(rc, clone) {
  if (rc.strong === 1) return $rcTake(rc);
  rc.strong--;
  return clone ? clone(rc.value) : rc.value;
}

// `Rc::make_mut(&mut rc)`: the `Rc` to change, itself if it's the only one
// and nothing's weak to it, else a new one of what it pointed at, moved if
// it was the last strong one, else cloned.
function $makeMut(rc, clone) {
  if (rc.strong === 1) {
    if (rc.weak === 0) return rc;
    return { value: $rcTake(rc), strong: 1, weak: 0 };
  }
  rc.strong--;
  return { value: clone ? clone(rc.value) : rc.value, strong: 1, weak: 0 };
}

// `Rc::new_cyclic(f)`: `f` given a `Weak` to what it makes, which can't
// upgrade until it's made.
function $newCyclic(f) {
  const rc = { value: undefined, strong: 0, weak: 1 };
  rc.value = f(rc);
  rc.strong = 1;
  rc.weak--;
  return rc;
}
