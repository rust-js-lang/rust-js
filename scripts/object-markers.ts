// What a binding of a JS class's instances is to the js crate, as an object
// always is: never `undefined` nor `null`, `Defined`, so an `Option` of one
// is the object or `undefined` (ADR 0051), and what a `WeakMap` or a
// `WeakSet` holds, `WeakKey` (ADR 0677). Each binding generator writes them
// of each class it binds by this, webapi's of each interface and the
// builtins' of each class and buffer; test/object-markers.test.ts holds
// every crate's `Classed` type, written by hand too, to them (ADR 1172,
// amended).

/** The markers every JS object's binding implements. */
export const OBJECT_MARKERS = ["Defined", "WeakKey"] as const;

/** The lines that say `type` is an object's binding: `js` the path to the
 * js crate's root, `js::` or `super::`; `params` its impls' generics,
 * `<B>`. */
export const objectMarkers = (type: string, js: string, params = "") =>
  OBJECT_MARKERS.map((marker) => `unsafe impl${params} ${js}${marker} for ${type} {}`);
