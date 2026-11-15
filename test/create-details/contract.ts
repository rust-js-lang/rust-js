// Codex's runtime contract of createDetails, verbatim from
// rust-js-lang/vercel-ui-sdk-in-rust@82369d8, tests/change-details.test.ts:
// spread, cancellation, descriptors, thrown identity, and each getter's and
// Proxy trap's order. Each row it returns of the compiled function must be
// upstream's.
import { expect } from "bun:test";

export type Factory = (given: object, source: string, cancelable?: boolean) => Record<PropertyKey, unknown>;

export function contract(create: Factory) {
  const rows: unknown[] = [];
  const token = Symbol.for("ui-sdk-test-change-details");
  for (const mode of [undefined, true, false]) {
    const extra = {count:1};
    const event = new Event("input");
    const given = {reason:"input-change", event, extra, source:"old", [token]:extra};
    const details = create(given, "agent", mode);
    const canceled = Object.getOwnPropertyDescriptor(details, "canceled");
    rows.push({mode, keys:Reflect.ownKeys(details), prototype:Object.getPrototypeOf(details) === Object.prototype,
      source:details.source, eventIdentity:details.event === event, extraIdentity:details.extra === extra,
      symbolIdentity:details[token] === extra, descriptor:canceled && {
        enumerable:canceled.enumerable, configurable:canceled.configurable,
        getter:typeof canceled.get, setter:typeof canceled.set, data:Object.hasOwn(canceled, "value"),
      }});
    expect(details).not.toBe(given);
    expect(given.source).toBe("old");
    given.extra = {count:2};
    expect(details.extra).toBe(extra);
    if (mode !== false) {
      const stop = details.cancel;
      expect(typeof stop).toBe("function");
      const states = [details.canceled];
      const results = [(stop as () => void).call({}), (stop as () => void)()];
      states.push(details.canceled);
      const next = create({reason:"press"}, "mouse");
      rows.push({states, results, independent:next.canceled, json:JSON.parse(JSON.stringify(details)).canceled});
      expect(states).toEqual([false, true]);
      expect(results).toEqual([undefined, undefined]);
      expect(next.canceled).toBe(false);
    } else {
      expect(Object.hasOwn(details, "cancel")).toBe(false);
      expect(Object.hasOwn(details, "canceled")).toBe(false);
    }
  }
  // The original trusts the reason/event relationship without checking classes.
  const unrelatedEvent = new Event("click");
  const asserted = create({reason:"escape-key", event:unrelatedEvent}, "mouse");
  expect(asserted.event).toBe(unrelatedEvent);
  const snapshot = {...asserted};
  (asserted.cancel as () => void)();
  expect(snapshot.canceled).toBe(false);
  expect(asserted.canceled).toBe(true);
  rows.push({uncheckedIdentity:asserted.event === unrelatedEvent, snapshot:snapshot.canceled, live:asserted.canceled});
  // Spread retains a present undefined event and omits an absent event.
  for (const given of [{reason:"custom"}, {reason:"custom", event:undefined}]) {
    const details = create(Object.freeze(given), "programmatic", false);
    rows.push({eventOwn:Object.hasOwn(details, "event"), keys:Reflect.ownKeys(details)});
  }
  // A noncancelable result still retains extra fields named cancel/canceled.
  const extraKeys = create({reason:"custom", cancel:17, canceled:"given"}, "touch", false);
  expect(extraKeys.cancel).toBe(17);
  expect(extraKeys.canceled).toBe("given");
  rows.push(extraKeys);
  // Inherited and nonenumerable fields are not copied or read.
  let inheritedReads = 0;
  const inherited = Object.create({get reason() {inheritedReads++; return "inherited";}});
  Object.defineProperty(inherited, "hidden", {get() {throw Error("hidden was read");}});
  inherited.extra = 7;
  const copied = create(inherited, "pen", false);
  expect(inheritedReads).toBe(0);
  rows.push({keys:Reflect.ownKeys(copied), reasonOwn:Object.hasOwn(copied, "reason"), hiddenOwn:Object.hasOwn(copied, "hidden")});
  // Own enumerable getters run even where source/cancel/canceled replace them.
  const names = ["reason", "event", "extra", "source", "cancel", "canceled"];
  for (const mode of [true, false]) for (const stop of [undefined, ...names]) {
    const calls: string[] = [];
    const marker = {};
    const given = Object.defineProperties({}, Object.fromEntries(names.map(name => [name, {
      enumerable:true, get() {calls.push(name); if (name === stop) throw marker; return name === "reason" ? "press" : name;},
    }])));
    let details: Record<PropertyKey, unknown> | undefined;
    let thrown = false;
    try {details = create(given, "keyboard", mode);} catch (error) {thrown = error === marker;}
    rows.push({mode, stop, calls, thrown, keys:details && Reflect.ownKeys(details), source:details?.source});
    expect(calls).toEqual(stop === undefined ? names : names.slice(0, names.indexOf(stop) + 1));
    expect(thrown).toBe(stop !== undefined);
  }
  // Object spread's proxy protocol and string/symbol ordering stay observable.
  const calls: string[] = [];
  const describe = (key: PropertyKey) => typeof key === "symbol" ? "symbol" : String(key);
  const given = new Proxy({reason:"press", "2":"two", "1":"one", [token]:9}, {
    ownKeys(target) {calls.push("keys"); return Reflect.ownKeys(target);},
    getOwnPropertyDescriptor(target, key) {calls.push("descriptor:" + describe(key)); return Reflect.getOwnPropertyDescriptor(target, key);},
    get(target, key, receiver) {calls.push("get:" + describe(key)); return Reflect.get(target, key, receiver);},
  });
  const details = create(given, "virtual", false);
  rows.push({proxyCalls:calls, keys:Reflect.ownKeys(details), value:details[token]});
  expect(calls).toEqual(["keys", "descriptor:1", "get:1", "descriptor:2", "get:2", "descriptor:reason", "get:reason", "descriptor:symbol", "get:symbol"]);
  return rows;
}
