// The generator and the reducer (ADR 0092), without running what they make.

import { expect, test } from "bun:test";

import { generate, print, random, reduce, size, type Program } from "./generate";

test("a seed is the same program, and another seed another", () => {
  expect(print(generate(7))).toBe(print(generate(7)));
  const programs = new Set(Array.from({ length: 50 }, (_, seed) => print(generate(seed))));
  expect(programs.size).toBe(50);
  // The same numbers from the same seed, and in [0, 1).
  const a = random(3), b = random(3);
  const draws = Array.from({ length: 100 }, () => a.next());
  expect(draws).toEqual(Array.from({ length: 100 }, () => b.next()));
  expect(draws.every((x) => x >= 0 && x < 1)).toBe(true);
});

// A text a loop appends to, of itself or of another text that grows too,
// doubles each turn until the program runs out of memory: seed 1311's
// \`v4.push_str(v4.clone())\`, nested in loops, took CI's runner down.
test("what's appended to a text reads no text", () => {
  for (let seed = 0; seed < 2000; seed++) {
    const text = print(generate(seed), seed);
    const texts = [...text.matchAll(/let (?:mut )?(\w+): String\b/g)].map((m) => m[1]);
    for (const [, argument] of text.matchAll(/\.push_str\((.*)\.as_str\(\)\);$/gm)) {
      const read = texts.filter((name) => new RegExp(`\\b${name}\\b`).test(argument));
      expect([seed, read]).toEqual([seed, []]);
    }
  }
});

// A NaN's sign isn't kept (the guide's differences, 4; ADR 0154), and a
// NaN an operation makes has its sign set on x86: seed 12449569's
// `(-id(f64::NAN)).is_sign_negative()` was `true` natively, `false` as JS.
test("a float's sign is asked only of a number", () => {
  let asked = 0;
  for (let seed = 0; seed < 2000; seed++) {
    const text = print(generate(seed), seed);
    const all = text.match(/\.is_sign_negative\(\)/g) ?? [];
    const ofNumbers = text.match(/!f\.is_nan\(\) && f\.is_sign_negative\(\)/g) ?? [];
    expect([seed, ofNumbers.length]).toEqual([seed, all.length]);
    asked += all.length;
  }
  expect(asked).toBeGreaterThan(20);
  expect(print(generate(12449569), 12449569)).toContain("!f.is_nan() && f.is_sign_negative() }");
});

test("a program is `main` and `id`, which hides each literal from rustc", () => {
  const text = print(generate(1), 1);
  expect(text).toStartWith("// Generated from seed 1.\nfn id<T>(x: T) -> T {");
  expect(text).toContain("#[allow(unused, arithmetic_overflow, unconditional_panic)]\nfn main() {");
  // Every typed literal is `id(..)`'s, so rustc can't work out an overflow.
  const literals = text.match(/-?\b\d+[iu](8|16|32|64)\b/g) ?? [];
  const hidden = text.match(/id\(-?\d+[iu](8|16|32|64)\)/g) ?? [];
  expect(literals.length).toBeGreaterThan(5);
  expect(hidden.length).toBe(literals.length);
});

const seven = { kind: "lit", ty: "i32", value: 7n } as const;
const one = { kind: "lit", ty: "i32", value: 1n } as const;

test("a failing program is reduced to as little as still fails", async () => {
  // Fails while it prints 7, wherever that is.
  const program: Program = [
    { kind: "let", name: "v0", ty: "i32", value: one },
    { kind: "print", format: "{}", value: one },
    {
      kind: "for",
      name: "k1",
      n: 2,
      body: [
        { kind: "if", c: { kind: "lit", ty: "bool", value: true }, then: [{ kind: "print", format: "{}", value: { kind: "bin", ty: "i32", op: "+", a: seven, b: one } }], else: [] },
      ],
    },
    { kind: "print", format: "{:x}", value: one },
  ];
  let tried = 0;
  const kept: Program[] = [];
  const { program: smallest, complete } = await reduce(
    program,
    async (candidate) => {
      tried++;
      return print(candidate).includes("id(7i32)");
    },
    { kept: (p) => kept.push(p) },
  );
  expect(complete).toBe(true);
  // Each smaller one, as it's found, the last the smallest.
  expect(kept.at(-1)).toEqual(smallest);
  expect(kept.map((p) => print(p).length)).toEqual(kept.map((p) => print(p).length).sort((a, b) => b - a));
  expect(size(program)).toBe(6);
  // The loop and the `if` around it, and the `+` in it, are gone.
  expect(smallest).toEqual([{ kind: "print", format: "{}", value: seven }]);
  expect(tried).toBeGreaterThan(5);
});

test("a program that fails only as it is stays as it is", async () => {
  const program = generate(5);
  const text = print(program);
  expect(await reduce(program, async (candidate) => print(candidate) === text)).toEqual({ program, complete: true });
});

// A reduction that runs out of time keeps what it found, and says it
// isn't done. Found in review: a reduction cut short lost the failure.
test("a reduction cut short is the smallest found so far, and says so", async () => {
  const program = generate(5);
  const kept: Program[] = [];
  let calls = 0;
  const until = Date.now() + 50;
  const { program: smallest, complete } = await reduce(
    program,
    async () => {
      calls++;
      await Bun.sleep(20);
      return true;
    },
    { until, kept: (p) => kept.push(p) },
  );
  expect(complete).toBe(false);
  expect(calls).toBeGreaterThan(0);
  expect(kept.length).toBe(calls);
  expect(smallest).toEqual(kept.at(-1)!);
  expect(size(smallest)).toBeLessThan(size(program));
});
