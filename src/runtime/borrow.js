// A `RefCell`'s `borrow()` and `borrow_mut()` (ADR 0328): the cell, if it's
// free to borrow so, as its guard. `borrows` counts the guards held: how
// many share it, or -1 for the one that changes it. One held only while
// nothing can ask isn't counted; with `hold`, it's counted until `$unborrow`.
// A `RefCell` of an object is the object, which counts its own (ADR 0863).
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

// `try_lock()`, `try_write()` (`mutable`) and `try_read()`: `Ok` of a guard
// held, or, while the thread holds it so, `Err` of `TryLockError::WouldBlock`,
// where `lock()` would deadlock.
function $tryLock(cell, mutable) {
  if (mutable ? cell.borrows : cell.borrows < 0) return { TAG: "Err", _0: "WouldBlock" };
  return { TAG: "Ok", _0: mutable ? $lock(cell, true) : $lockRead(cell, true) };
}

// A `Mutex`'s or an `RwLock`'s `{:?}`: what it holds, shown by `show`
// where it could be locked to read it, a `Mutex`'s by anything holding it,
// an `RwLock`'s by a writer, else that it's locked, as std's `try_lock` and
// `try_read` say, a `Mutex`'s as a string, `"<locked>"`, as std's shows it;
// never poisoned (ADR 0328).
function $showLocked(cell, show, name, exclusive) {
  const locked = exclusive ? cell.borrows : cell.borrows < 0;
  const data = !locked ? $withBorrow(cell, show) : exclusive ? '"<locked>"' : "<locked>";
  return `${name} { data: ${data}, poisoned: false, .. }`;
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
