// Mutations of scripts/object-markers.ts: what every JS object's binding
// is, which each binding generator writes of each class (ADR 1172, amended).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "object-markers-weak-key-forgotten",
    breaks: "a generated class's instances aren't `WeakKey`s, so a `WeakMap` keyed by an `Intl.Locale` or a typed array is refused",
    file: "scripts/object-markers.ts",
    find: 'export const OBJECT_MARKERS = ["Defined", "WeakKey"] as const;',
    replace: 'export const OBJECT_MARKERS = ["Defined"] as const;',
    tests: ["test/builtins-coverage.test.ts", "-t", "Intl is what its template writes"],
  },
];
