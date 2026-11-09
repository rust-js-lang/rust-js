// Swarm testing of the generator's programs (ADR 0092, amended, #1085):
// the families of statement `block` chooses among, and the ones a seed's
// program has.

import { random, type Random } from "./generate";

/** The families of statements a program may have: one seed's are some of
 * them (`swarm`), so a program of few kinds of statement has more of each,
 * as swarm testing makes them (Groce et al., ISSTA 2012; #1085). */
export const families = ["assign", "vecs", "sets", "closures", "branches", "traits", "text", "maps", "loops", "shared", "tuples", "drops", "places"] as const;
export type Family = (typeof families)[number] | "core";
export type Counter = { n: number; swarm: Set<Family> };

// The statements `block` chooses among: its 30, and two of `Pl`s.
export const choices = 32;

/** The family of `block`'s statement `choice`. */
export function familyOf(choice: number): Family {
  if (choice < 3 || (choice >= 5 && choice < 8)) return "core";
  if (choice < 5) return "assign";
  if (choice === 8 || choice === 25 || choice === 28) return "vecs";
  if (choice === 9) return "sets";
  if (choice === 10 || choice === 18) return "closures";
  if (choice === 11 || choice === 12 || choice === 17) return "branches";
  if (choice === 13) return "traits";
  if (choice === 14) return "text";
  if (choice === 15 || choice === 16) return "maps";
  if (choice === 24) return "shared";
  if (choice === 26 || choice === 27) return "tuples";
  if (choice >= 19 && choice <= 21) return "drops";
  if (choice === 29) return "drops";
  if (choice >= 30) return "places";
  return "loops";
}

/** The families seed `r`'s program has: each one of `families` by a coin,
 * none of them left out by every seed. */
export function swarm(r: Random): Set<Family> {
  return new Set<Family>(["core", ...families.filter(() => r.chance(0.6))]);
}

/** The families seed `seed`'s program has (`swarm`), its first draws. */
export const swarmOf = (seed: number): Set<Family> => swarm(random(seed));
