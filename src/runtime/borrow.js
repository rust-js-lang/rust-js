// A `RefCell`'s `borrow()` and `borrow_mut()` (ADR 0328): the cell, if it's
// free to borrow so, as its guard. `borrows` counts the guards held: how
// many share it, or -1 for the one that changes it. One held only while
// nothing can ask isn't counted; with `hold`, it's counted until `$unborrow`.
// A `RefCell` of an object is the object, which counts its own (ADR 0863).
// A `RefCell` found borrowed where a borrow is taken, std's panic: its
// mutable borrow's, where it's borrowed at all, and its shared one's,
// where it's borrowed mutably. A momentary borrow, held only while
// nothing can ask, is checked where it's taken, `if (cell.borrows)
// $alreadyBorrowed();`, with no call where the cell is free (rd218, #1692).
const $alreadyBorrowedMessage = "RefCell already borrowed";
function $alreadyBorrowed() {
  throw new Error($alreadyBorrowedMessage);
}
function $alreadyMutablyBorrowed() {
  throw new Error("RefCell already mutably borrowed");
}

function $borrow(cell, hold) {
  if (cell.borrows < 0) $alreadyMutablyBorrowed();
  if (hold) $setBorrows(cell, (cell.borrows ?? 0) + 1);
  return cell;
}

function $borrowMut(cell, hold, dropGiven) {
  if (cell.borrows) {
    // What the call was given, a `replace`'s value, dropped as its panic
    // unwinds, as Rust's parameter is (#1587): a cleanup of that panic, so
    // `thread::panicking()` holds in its drop, and a panic there aborts.
    const error = new Error($alreadyBorrowedMessage);
    if (dropGiven !== undefined) $cleanup(dropGiven, error);
    throw error;
  }
  if (hold) $setBorrows(cell, -1);
  return cell;
}

// A guard dropped: one borrow fewer. Of a guard `$guardMap` gave, what it
// owes: of its cell, one borrow fewer where it's the last of a ticket's
// holders, `map_split`'s two (#1275).
function $unborrow(cell) {
  const tickets = $guardTickets.get(cell);
  if (tickets !== undefined) {
    const ticket = tickets.pop();
    if (tickets.length === 0) $guardTickets.delete(cell);
    if (--ticket.holders.left === 0) $unborrow(ticket.cell);
    return;
  }
  $setBorrows(cell, cell.borrows > 0 ? cell.borrows - 1 : 0);
}

// What each guard `$guardMap` gave owes, by the guard: a ticket of each
// borrow it holds, `{ cell, holders }`, the cell whose borrow it is and
// how many guards share it, each one fewer as it drops (#1275). A guard
// that's its part itself, an object, may be given more than once, each a
// ticket, of one cell: two cells' guards of one object can't be told
// apart, refused as they're made.
const $guardTickets = new WeakMap();

// The ticket `guard` held, it taken: a mapped one's, or of its cell, one
// guard's.
function $guardTicket(guard) {
  const tickets = typeof guard === "object" && guard !== null ? $guardTickets.get(guard) : undefined;
  if (tickets === undefined) return { cell: guard, holders: { left: 1 } };
  const ticket = tickets.pop();
  if (tickets.length === 0) $guardTickets.delete(guard);
  return ticket;
}

// A guard of a part, `made`, in its guard's form, holding `ticket`: the
// object itself (`object`), a box writing through the handle `made` is
// (`box`), or a `{ value }`; a guard of a cell, whose own guards a
// mapped one's release would meet, refused (#1307).
function $guardOf(made, form, ticket) {
  if (form === "cell") {
    throw $unsupported("rust-js does not support a guard mapped to a cell, whose own guards would release what the mapped one holds (#1307)");
  }
  const guard =
    form === "object"
      ? made
      : form === "box"
        ? {
            get value() {
              return made.value;
            },
            set value(value) {
              made.value = value;
            },
          }
        : { value: made };
  // Of a borrow nothing counted, a field's no borrow of which can meet
  // another, nothing owed (ADR 0863).
  if (typeof ticket.cell !== "object" || ticket.cell === null) return guard;
  const tickets = $guardTickets.get(guard);
  if (tickets === undefined) $guardTickets.set(guard, [ticket]);
  else if (tickets[0].cell === ticket.cell) tickets.push(ticket);
  else throw $unsupported("rust-js does not support two cells' guards mapped to one object, whose releases it can't tell apart (#1307)");
  return guard;
}

// `Ref::map(guard, f)` and `RefMut::map`: a guard of what `made`, `f` of
// what `guard` guards, gives, in `form`, which releases what `guard`
// held as it drops; where `f` panics, `guard` released first, as std
// drops it as the panic unwinds (#1275).
function $guardMap(guard, made, form) {
  const part = $unwinds(made, () => $unborrow(guard));
  return $guardOf(part, form, $guardTicket(guard));
}

// `Ref::filter_map(guard, f)`: `Ok` of a guard of what `f` gives, or of
// `None`, `Err` of `guard`, still held.
function $guardFilterMap(guard, made, form) {
  const part = $unwinds(made, () => $unborrow(guard));
  if (part === undefined || part === null) return { TAG: "Err", _0: guard };
  return { TAG: "Ok", _0: $guardOf(part, form, $guardTicket(guard)) };
}

// `Ref::map_split(guard, f)`: a guard of each of the two parts `f` gives,
// which share what `guard` held, released as the second drops.
function $guardMapSplit(guard, made, a, b) {
  const [first, second] = $unwinds(made, () => $unborrow(guard));
  const ticket = $guardTicket(guard);
  ticket.holders.left += 1;
  return [$guardOf(first, a, ticket), $guardOf(second, b, ticket)];
}

// Not enumerable: `==`, a clone and JSON see the cell's `value` alone.
function $setBorrows(cell, borrows) {
  Object.defineProperty(cell, "borrows", { value: borrows, writable: true, configurable: true });
}

// A struct's `RefCell` field that holds its value, `object[key]`, some
// borrow of which is checked (ADR 0863, amended): the one cell of it there
// is, made where it's first asked, which counts its borrows as a
// `{ value }` does, its `value` the property; of a nullable field, its
// `None` `null` where it's written.
const $fieldCells = new WeakMap();
// Each such cell's field, its object and key, where a pointer to what it
// lends is (#1114).
const $cellFields = new WeakMap();

function $fieldCell(object, key, nullable) {
  let cells = $fieldCells.get(object);
  if (cells === undefined) $fieldCells.set(object, (cells = new Map()));
  let cell = cells.get(key);
  if (cell === undefined) {
    cell = {
      get value() {
        return object[key];
      },
      set value(value) {
        object[key] = nullable ? (value ?? null) : value;
      },
    };
    cells.set(key, cell);
    $cellFields.set(cell, { object, key });
  }
  return cell;
}

// A `#[rust_js::nullable]` thread-local's cell, kept as a cell where it's
// lent whole (#541): its `None` `null` however it's written, through a
// `with`'s `&RefCell` or a `with_borrow_mut`'s `&mut` too, as its reads
// take it; its `value` alone seen, as a `{ value }`'s is.
function $nullableCell(value) {
  return {
    get value() {
      return value;
    },
    set value(given) {
      value = given ?? null;
    },
  };
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
// it before what it drops can ask. Of one that's the object it holds
// (`itself`, ADR 0863), the cell.
function $withBorrow(cell, f, itself) {
  $borrow(cell, true);
  try {
    return f(itself ? cell : cell.value);
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
function $showBorrowed(cell, show, itself) {
  return cell.borrows < 0 ? "<borrowed>" : $withBorrow(cell, show, itself);
}

// A `RefCell`'s `{:?}` into a writer's sink `f`: what it holds, written
// by `write(f, value)` while it's borrowed, or `<borrowed>`, one
// `write_str`, where it's mutably borrowed (#1652).
function $writeBorrowed(f, cell, write, itself) {
  if (cell.borrows < 0) return f.write("<borrowed>");
  $withBorrow(cell, (value) => write(f, value), itself);
}

// A `RefCell`'s `replace_with(f)`: `f` given a `&mut` to its value while
// it's mutably borrowed, the cell itself for a number or text (ADR 0074),
// and what it makes in its place. Of one that's the object it holds
// (`itself`, ADR 0863), the object is the `&mut`, changed in place.
function $replaceWith(cell, f, boxed, itself) {
  const next = $withBorrowMut(cell, f, boxed || itself);
  if (itself) return $take(cell, next);
  const previous = cell.value;
  cell.value = next;
  return previous;
}

// A `RefCell`'s `swap(&other)`: each mutably borrowed, the first while the
// second is, so a cell swapped with itself is borrowed twice. Two that are
// the objects they hold (`itself`, ADR 0863) swap what's in them.
function $refCellSwap(a, b, itself) {
  $borrowMut(a, true);
  try {
    $borrowMut(b);
  } finally {
    $unborrow(a);
  }
  if (itself) {
    $assign(b, $take(a, b));
  } else {
    [a.value, b.value] = [b.value, a.value];
  }
}

// The locks a panic poisoned, as std's are (#1438): each `Mutex` or
// `RwLock` whose writing guard was dropped as a panic unwound, where none
// unwound as it was taken, until its `clear_poison()`. And those whose
// writing guard was taken as one unwound, in a drop it runs, whose drop
// poisons nothing.
const $poisonedLocks = new WeakSet();
const $lockedUnwinding = new WeakSet();

// A `Mutex`'s `lock()`, an `RwLock`'s `write()` (`mutable`) and its
// `read()`: `Ok` of the guard, as a `RefCell`'s, held; or, of a poisoned
// lock, `Err` of a `PoisonError` that holds it. On one thread, locking
// what the thread holds deadlocks in Rust.
function $locked(cell, mutable) {
  const guard = $lockHeld(cell, mutable);
  return $poisonedLocks.has(cell) ? { TAG: "Err", _0: { data: guard } } : { TAG: "Ok", _0: guard };
}

// What `lock().unwrap()` and `write().unwrap()` give, and `read().unwrap()`,
// or with `expect(message)`, its message: the guard, held; or, of a
// poisoned lock, `unwrap`'s panic, the guard it held released as it
// unwinds, as the lock was.
function $lock(cell, message) {
  return $unpoisoned($lockHeld(cell, true), message, true);
}

function $lockRead(cell, message) {
  return $unpoisoned($lockHeld(cell, false), message, true);
}

function $lockHeld(cell, mutable) {
  if (mutable ? cell.borrows : cell.borrows < 0) $deadlock();
  if (!mutable) {
    $setBorrows(cell, (cell.borrows ?? 0) + 1);
  } else {
    $setBorrows(cell, -1);
    if ($panicking()) {
      $lockedUnwinding.add(cell);
    } else {
      $lockedUnwinding.delete(cell);
    }
  }
  return cell;
}

// A lock, `cell`, where it isn't poisoned; else `unwrap`'s panic, or
// `expect(message)`'s, of the `PoisonError` that holds its guard, which is
// released as the panic unwinds where it's `held`.
function $unpoisoned(cell, message = "called `Result::unwrap()` on an `Err` value", held = false) {
  if (!$poisonedLocks.has(cell)) return cell;
  if (held) $unlock(cell);
  throw new Error(`${message}: PoisonError { .. }`);
}

// A `Mutex`'s or an `RwLock`'s `into_inner()` and `get_mut()`: `Ok` of
// what it holds, or, of a poisoned lock, `Err` of a `PoisonError` that
// holds it.
function $lockedValue(cell) {
  return $poisonedLocks.has(cell) ? { TAG: "Err", _0: { data: cell.value } } : { TAG: "Ok", _0: cell.value };
}

// A `Mutex`'s guard or an `RwLock`'s writing one dropped: one borrow
// fewer, and, where a panic unwinds that didn't as it was taken, its lock
// poisoned, as std's (#1438). Only a cleanup a panic runs tells that
// (`$panicking`), so it's never in a `finally`, which runs either way.
function $unlock(cell) {
  if ($lockedUnwinding.has(cell)) {
    $lockedUnwinding.delete(cell);
  } else if ($panicking()) {
    $poisonedLocks.add(cell);
  }
  $unborrow(cell);
}

// A lock's guard held by a scope whose `try` a panic unwinds, its `catch`:
// its lock poisoned where no panic unwound as it was taken; the scope's
// `finally` releases it (`$unborrow`, #1438).
function $poison(cell) {
  if (!$lockedUnwinding.has(cell)) $poisonedLocks.add(cell);
}

// A lock's `is_poisoned()` and `clear_poison()`.
function $isPoisoned(cell) {
  return $poisonedLocks.has(cell);
}

function $clearPoison(cell) {
  $poisonedLocks.delete(cell);
}

// `try_lock()`, `try_write()` (`mutable`) and `try_read()`: `Ok` of a guard
// held; while the thread holds it so, `Err` of `TryLockError::WouldBlock`,
// where `lock()` would deadlock; or, of a poisoned lock, `Err` of
// `TryLockError::Poisoned` of the `PoisonError` that holds the guard.
function $tryLock(cell, mutable) {
  if (mutable ? cell.borrows : cell.borrows < 0) return { TAG: "Err", _0: "WouldBlock" };
  const locked = $locked(cell, mutable);
  return locked.TAG === "Ok" ? locked : { TAG: "Err", _0: { TAG: "Poisoned", _0: locked._0 } };
}

// A `Mutex`'s or an `RwLock`'s `{:?}`: what it holds, shown by `show`
// where it could be locked to read it, a `Mutex`'s by anything holding it,
// an `RwLock`'s by a writer, else that it's locked, as std's `try_lock` and
// `try_read` say, a `Mutex`'s as a string, `"<locked>"`, as std's shows it;
// and whether it's poisoned; `{:#?}`'s (`alternate`) a field a line, as
// std's `debug_struct` writes them (#1629).
function $showLocked(cell, show, name, exclusive, alternate) {
  const fields = [`data: ${$lockedData(cell, show, exclusive)}`, `poisoned: ${$lockPoisoned(cell)}`];
  return alternate ? $pretty(`${name} {`, fields, "}", true) : `${name} { ${fields.join(", ")}, .. }`;
}

// A `Mutex`'s or an `RwLock`'s `data` field, as `$showLocked` shows it, and
// as std's builder takes it where a part may stop part-way (#1530): what it
// holds, shown by `show` while it's locked to read it, else that it's locked.
// A `Mutex`'s guard, `try_lock`'s, held as a panic unwinds out of `show`,
// poisons it, as std's does; an `RwLock`'s reading one doesn't (#1631).
function $lockedData(cell, show, exclusive) {
  const locked = exclusive ? cell.borrows : cell.borrows < 0;
  if (locked) return exclusive ? '"<locked>"' : "<locked>";
  try {
    return $withBorrow(cell, show);
  } catch (error) {
    // A panic, as a cleanup it runs, not a write's `Err`, which returns.
    if (exclusive && !(error instanceof $WriteError)) $cleanup(() => $poison(cell), error);
    throw error;
  }
}

// Whether a panic poisoned a `Mutex` or an `RwLock`, its `poisoned` field.
function $lockPoisoned(cell) {
  return $poisonedLocks.has(cell);
}

function $deadlock() {
  throw $unsupported("rust-js does not support locking a lock its thread holds, which deadlocks in Rust");
}

// A `RefCell<C>` as a `RefCell<dyn T>` (ADR 0397): the same cell, given the
// dictionary of `C`'s `T`, so it's the pair a `dyn` is, its guard too. One
// made a `dyn` of another trait already has both's methods. Not enumerable,
// as `borrows` isn't.
function $dynCell(cell, impl) {
  const held = cell.impl;
  const both = held === undefined || held === impl ? impl : { ...held, ...impl };
  Object.defineProperty(cell, "impl", { value: both, writable: true, configurable: true });
  return cell;
}
