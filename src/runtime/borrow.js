// A `RefCell`'s `borrow()` and `borrow_mut()` (ADR 0328): the cell, if it's
// free to borrow so, as its guard. `borrows` counts the guards held: how
// many share it, or -1 for the one that changes it. One held only while
// nothing can ask isn't counted; with `hold`, it's counted until `$unborrow`.
function $borrow(cell, hold) {
  if (cell.borrows < 0) throw new Error("RefCell already mutably borrowed");
  if (hold) $setBorrows(cell, (cell.borrows ?? 0) + 1);
  return cell;
}

function $borrowMut(cell, hold) {
  if (cell.borrows) throw new Error("RefCell already borrowed");
  if (hold) $setBorrows(cell, -1);
  return cell;
}

// A guard dropped: one borrow fewer.
function $unborrow(cell) {
  $setBorrows(cell, cell.borrows > 0 ? cell.borrows - 1 : 0);
}

// Not enumerable: `==`, a clone and JSON see the cell's `value` alone.
function $setBorrows(cell, borrows) {
  Object.defineProperty(cell, "borrows", { value: borrows, writable: true, configurable: true });
}

// `try_borrow()`, `try_borrow_mut()`: `Ok` of a guard held, or `Err` of
// the error, which is its message.
function $tryBorrow(cell, mutable) {
  if (mutable ? cell.borrows : cell.borrows < 0) {
    return { TAG: "Err", _0: mutable ? "RefCell already borrowed" : "RefCell already mutably borrowed" };
  }
  return { TAG: "Ok", _0: mutable ? $borrowMut(cell, true) : $borrow(cell, true) };
}

// What `f` makes of a `RefCell`'s value while it's borrowed: its clone, or
// whether it's equal, where `f` may ask too. A panic's unwinding releases
// it before what it drops can ask.
function $withBorrow(cell, f) {
  $borrow(cell, true);
  try {
    return f(cell.value);
  } finally {
    $unborrow(cell);
  }
}

// What `f` makes of a `&mut` to a `RefCell`'s value while it's mutably
// borrowed, the cell itself for a number or text (ADR 0074): a
// thread-local's `with_borrow_mut(f)`.
function $withBorrowMut(cell, f, boxed) {
  $borrowMut(cell, true);
  try {
    return f(boxed ? cell : cell.value);
  } finally {
    $unborrow(cell);
  }
}

// A `RefCell`'s `{:?}`: what it holds, shown by `show` while it's borrowed,
// or `<borrowed>` where it's mutably borrowed.
function $showBorrowed(cell, show) {
  return cell.borrows < 0 ? "<borrowed>" : $withBorrow(cell, show);
}

// A `RefCell`'s `replace_with(f)`: `f` given a `&mut` to its value while
// it's mutably borrowed, the cell itself for a number or text (ADR 0074),
// and what it makes in its place.
function $replaceWith(cell, f, boxed) {
  const next = $withBorrowMut(cell, f, boxed);
  const previous = cell.value;
  cell.value = next;
  return previous;
}

// A `RefCell`'s `swap(&other)`: each mutably borrowed, the first while the
// second is, so a cell swapped with itself is borrowed twice.
function $refCellSwap(a, b) {
  $borrowMut(a, true);
  try {
    $borrowMut(b);
  } finally {
    $unborrow(a);
  }
  [a.value, b.value] = [b.value, a.value];
}

// A `Mutex`'s `lock()`, an `RwLock`'s `write()`, and its `read()`: the
// guard, as a `RefCell`'s. On one thread, locking what the thread holds
// deadlocks in Rust.
function $lock(cell, hold) {
  if (cell.borrows) $deadlock();
  if (hold) $setBorrows(cell, -1);
  return cell;
}

function $lockRead(cell, hold) {
  if (cell.borrows < 0) $deadlock();
  if (hold) $setBorrows(cell, (cell.borrows ?? 0) + 1);
  return cell;
}

function $deadlock() {
  throw new Error("rust-js does not support locking a lock its thread holds, which deadlocks in Rust");
}
