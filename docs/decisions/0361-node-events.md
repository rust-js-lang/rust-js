# 0361. Node's events are names of their own, typed by their emitters

Status: Accepted. Extends [0272](0272-node.md); adapts webapi's
[0223](0223-webapi-event-and-tag-maps.md) and
[0282](0282-event-listener-methods.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Most of Node's classes are emitters: `process`, a stream, a server, a
child process. @types/node types each event by overloads of `on`, about
1,700 lines of `on(event: "exit", listener: (code: number) => void): this`,
and a program's own `EventEmitter` by a map of its events. Their listeners
take several arguments, `(req, res)`, where the DOM's take one event.

- **webapi** (ADRs 0223, 0282) types the DOM's events by marker structs, a
  string in JS, `events::Click` of `"click"`, and `Listen<E>`, which says a
  target's event's type: `add_event_listener(events::Click, |e| ..)`.
- **ReScript's Node bindings** write a method per event, `onData`,
  `onExit`, of `on` with the name fixed; an open emitter takes a typed key,
  `Event.t<'listener, 'emitter>`.

## Decision

**An event is a marker struct, its name in JS, and an emitter says each of
its events' listener's and arguments' types by `Emits<E>`:**

```rust
pub trait Emits<E> {
    type Listener: ?Sized; // dyn Fn(f64)
    type Args;             // (f64,)
}

process::on(event::Exit, Box::new(|code| println!("{code}")));
emitter.emit(Ping, (1.0, "a"));
```

```js
process.on("exit", (code) => { console.log(`${code}`); });
emitter.emit("ping", 1, "a");
```

- **A listener is a `Box` of its `dyn Fn`**, so a closure's parameters are
  its event's types, as rustc infers them through the box; `off` takes the
  same function, `&'static`, which `events::listener` makes and
  `Box::new` gives `on` too.
- **`emit`'s arguments are a tuple**, spread: a variadic binding's last
  parameter may be one now (ADR 0221's amendment).
- **`EventEmitterExt`** has an emitter's methods, `on`, `once`, `off`,
  `emit`, `listener_count` and the rest, of the events it `Emits`, as
  webapi's `EventTargetExt` has a target's: each of Node's emitters
  implements it; `process`'s are functions of the global, of `Process`.
- **A program's own event is its own marker**, `#[rust_js::name = "ping"]
  struct Ping;`, and `impl Emits<Ping> for EventEmitter`: the orphan rule
  lets it, as `Ping` is the program's.
- **An enum of names is an event too**: `Signals`, each a signal's name,
  is `process`'s signal event, `process::on(Signals::Sigint, ..)`.
- **`events.once(emitter, event)`** gives a promise of the event's `Args`,
  the array JS resolves with, destructured as a tuple.

## Why

- **It's webapi's model**, which a Rust program already writes, with what
  Node's listeners need: several arguments, and `emit`.
- **Each call is checked**: an event the emitter hasn't, or a listener of
  other arguments, is rustc's error, as TypeScript's overloads are.
- **The JS is a person's**: `process.on("exit", (code) => ..)`.

## Alternatives

- **A method per event**, ReScript's `onExit`: a name for each of 1,700
  overloads' pairs, and none for a program's own events.
- **`on(name: &str, listener)`, untyped**: no listener's arguments to
  type, and a misspelled event only Node finds.
- **`impl Fn` listeners**: rustc can't infer a closure's parameters from
  an associated type's bound, so each would need its types written.

## Consequences

- `events` (2026-10-10): 11 of 16, and `process`'s listeners; streams',
  servers' and child processes' events follow as their classes are bound.
- `EventEmitter.captureRejections` and `defaultMaxListeners` are written
  through the class, `set events#EventEmitter.defaultMaxListeners`: a
  binding writes an import's property now, as a global's (ADR 0024's
  amendment).
- An event's name as a symbol, `errorMonitor`, isn't an event marker yet.
