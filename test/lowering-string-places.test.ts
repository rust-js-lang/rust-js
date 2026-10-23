// What the compiler writes of a string's offsets that are only places in
// the string they were found in (ADR 0642): JS's own `indexOf`, `slice`,
// `startsWith` and `length`, an `Option` found `-1` where it's `None`; and
// ADR 0138's helpers, exact in UTF-8 bytes, where the program can see the
// number. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run } from "./support";

beforeAll(buildReact, 600_000);

/** `source` compiled as a library, its JS and its module. */
async function compiled(name: string, source: string): Promise<{ js: string; lib: Record<string, (...args: unknown[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

// React's `formatOwnerStack`, written as std has it, reads as upstream's
// `shared/ReactOwnerStackFrames.js`: no helper, no byte offset.
test("React's owner stack reads as upstream's", async () => {
  const { js, lib } = await compiled("owner-stack", `pub fn format_owner_stack(stack: &str) -> String {
    let mut stack = stack;
    if let Some(rest) = stack.strip_prefix("Error: react-stack-top-frame\\n") {
        stack = rest;
    }
    if let Some(idx) = stack.find('\\n') {
        // Pop the JSX frame.
        stack = &stack[idx + 1..];
    }
    let idx = stack
        .find("react_stack_bottom_frame")
        .and_then(|idx| stack[..idx].rfind('\\n'));
    if let Some(idx) = idx {
        stack = &stack[..idx];
    } else {
        return String::new();
    }
    stack.to_string()
}
`);
  expect(js).toContain(`export function formatOwnerStack(stack) {
  if (stack.startsWith("Error: react-stack-top-frame\\n")) {
    stack = stack.slice(29);
  }
  let idx = stack.indexOf("\\n");
  if (idx !== -1) {
    // Pop the JSX frame.
    stack = stack.slice(idx + 1);
  }
  const at = stack.indexOf("react_stack_bottom_frame");
  idx = at !== -1 ? stack.slice(0, at).lastIndexOf("\\n") : -1;
  if (idx !== -1) {
    stack = stack.slice(0, idx);
    return stack;
  }
  return "";
}`);
  expect(js).not.toContain("@rust-js/runtime");
  const stack = "Error: react-stack-top-frame\n    at jsx\n    at é (a.js:1)\n    at 🦀 (b.js:2)\n    at react_stack_bottom_frame (r.js:3)";
  expect(lib.formatOwnerStack(stack)).toBe("    at é (a.js:1)\n    at 🦀 (b.js:2)");
  expect(lib.formatOwnerStack("no frames")).toBe("");
});

// A place stepped past what was found by the pattern's length: a literal's
// in units, `+ 1` of `'é'`'s two bytes, a string's or a `char`'s `length`.
test("a place stepped past what was found counts the pattern in units", async () => {
  const { js, lib } = await compiled("place-steps", `pub fn after_e(s: &str) -> &str {
    match s.find('é') {
        Some(at) => &s[at + 2..],
        None => s,
    }
}

pub fn after<'a>(s: &'a str, pattern: &str) -> &'a str {
    match s.find(pattern) {
        Some(at) => &s[at + pattern.len()..],
        None => "",
    }
}

pub fn after_char(s: &str, c: char) -> &str {
    match s.rfind(c) {
        Some(at) => &s[at + c.len_utf8()..],
        None => s,
    }
}

pub fn quoted(s: &str) -> &str {
    match s.find("«") {
        Some(open) => &s[open + "«".len()..s.len()],
        None => s,
    }
}
`);
  expect(js).toContain("return s.slice(at + 1);");
  expect(js).toContain("return s.slice(at + pattern.length);");
  expect(js).toContain("return s.slice(at + c.length);");
  expect(js).toContain('return s.slice(open + "«".length, s.length);');
  expect(js).not.toContain("@rust-js/runtime");
  expect(lib.afterE("🦀é🦀")).toBe("🦀");
  expect(lib.after("a🦀b🦀c", "🦀")).toBe("b🦀c");
  expect(lib.afterChar("é.🦀.x", ".")).toBe("x");
  expect(lib.afterChar("aé🦀b", "🦀")).toBe("b");
  expect(lib.quoted("x«é»")).toBe("é»");
});

// A strip whose `Option` is only tested and taken apart is a test and a
// cut, which count nothing: `startsWith` and `slice` by the pattern's length
// in units, of a literal or of what's given.
test("a strip tested and taken apart is startsWith and slice", async () => {
  const { js, lib } = await compiled("strips", `pub fn trimmed(s: &str) -> &str {
    if let Some(rest) = s.strip_suffix("🦀") {
        return rest;
    }
    s
}

pub fn stripped<'a>(s: &'a str, prefix: &str, suffix: char) -> &'a str {
    let mut s = s;
    if let Some(rest) = s.strip_prefix(prefix) {
        s = rest;
    }
    if let Some(rest) = s.strip_suffix(suffix) {
        s = rest;
    }
    s
}
`);
  expect(js).toContain(`  if (s.endsWith("🦀")) {
    return s.slice(0, -2);
  }`);
  expect(js).toContain(`  if (s.startsWith(prefix)) {
    s = s.slice(prefix.length);
  }
  if (s.endsWith(suffix)) {
    s = s.slice(0, s.length - suffix.length);
  }`);
  expect(js).not.toContain("@rust-js/runtime");
  expect([lib.trimmed("é🦀"), lib.trimmed("é")]).toEqual(["é", "é"]);
  expect([lib.stripped("é-🦀x", "é-", "x"), lib.stripped("abc", "", "é")]).toEqual(["🦀", "abc"]);
});

// An `Option` found given to a closure that takes it as a place is tested
// as `indexOf` says none: `map`'s `undefined` where it's `None`.
test("map of a place found tests it as indexOf does", async () => {
  const { js, lib } = await compiled("found-map", `pub fn after_colon(s: &str) -> Option<&str> {
    s.find(':').map(|at| &s[at + 1..])
}
`);
  expect(js).toContain('const at = s.indexOf(":");\n  return at !== -1 ? s.slice(at + 1) : undefined;');
  expect([lib.afterColon("é:🦀"), lib.afterColon("none")]).toEqual(["🦀", undefined]);
});

// Where the program can see an offset as a number, it's UTF-8 bytes, ADR
// 0138's: returned, compared with a number, used on another string,
// stepped by what isn't what was found, or unwrapped.
test("an offset the program can see keeps counting bytes", async () => {
  const { js, lib } = await compiled("offsets-seen", `pub fn found(s: &str) -> Option<usize> {
    s.find('é')
}

pub fn near_start(s: &str) -> bool {
    matches!(s.find('b'), Some(at) if at < 3)
}

pub fn on_another<'a>(s: &str, other: &'a str) -> &'a str {
    match s.find(' ') {
        Some(at) => &other[..at],
        None => other,
    }
}

pub fn stepped(s: &str) -> &str {
    match s.find('a') {
        Some(at) => &s[at + 3..],
        None => "",
    }
}

pub fn unwrapped(s: &str) -> &str {
    let at = s.find('=').unwrap();
    &s[..at]
}
`);
  expect(js).toContain('return $find(s, "é");');
  expect(js).toContain("at < 3");
  expect(js).toContain("return $strSlice(other, 0, at);");
  expect(js).toContain("return $strSlice(s, (at + 3) >>> 0);");
  expect(js).toContain("return $strSlice(s, 0, at);");
  expect([lib.found("🦀é"), lib.found("x")]).toEqual([4, undefined]);
  expect([lib.nearStart("ab"), lib.nearStart("ééb")]).toEqual([true, false]);
  expect(lib.onAnother("🦀 x", "abcdefgh")).toBe("abcd");
  expect(lib.stepped("xaé🦀yz")).toBe("🦀yz");
  expect(lib.unwrapped("🦀=1")).toBe("🦀");
});

// A place is of the string's value it was found in: the variable given
// another string after, the place, sliced there, is bytes, as Rust's is.
test("a place of a string a variable held before is bytes", async () => {
  const { js, lib } = await compiled("stale-place", `pub fn shifted(s: &str) -> &str {
    let mut s = s;
    let at = match s.find(' ') {
        Some(at) => at,
        None => return s,
    };
    s = &s[1..];
    &s[at..]
}
`);
  expect(js).toContain("$strSlice(s, at)");
  expect(js).not.toContain("indexOf");
  expect(lib.shifted("aé x")).toBe("x");
});

// A place found in a suffix is the suffix's, not the string's; a step past
// what was found by another string's length isn't a place; and a range of
// two places found apart may run backwards, which panics, as Rust's does.
test("a place is of what it was found in, past what was found", async () => {
  const { js, lib } = await compiled("places-apart", `pub fn in_suffix(s: &str) -> &str {
    if let Some(dot) = s.find('.') {
        let rest = &s[dot + 1..];
        if let Some(at) = rest.find('.') {
            return &s[..at];
        }
    }
    s
}

pub fn past_another<'a>(s: &'a str, p: &str, q: &str) -> &'a str {
    match s.find(p) {
        Some(at) => &s[at + q.len()..],
        None => s,
    }
}

pub fn between(s: &str) -> &str {
    if let Some(open) = s.find('(')
        && let Some(close) = s.find(')')
    {
        return &s[open + 1..close];
    }
    ""
}
`);
  expect(js).toContain("return $strSlice(s, 0, at);");
  expect(js).toContain("$strSlice(s, (at + $byteLen(q)) >>> 0)");
  expect(js).toContain("return $strSlice(s, (open + 1) >>> 0, close);");
  expect(lib.inSuffix("é.ab.c")).toBe("é");
  expect(lib.pastAnother("🦀ab", "a", "é")).toBe("");
  expect(lib.between("a(é)")).toBe("é");
  expect(() => lib.between(")(")).toThrow("byte range starts at 2 but ends at 0");
});

// A strip's `Option` read after its string's variable is given another:
// the string held where it's tested and cut, as Rust's is.
test("a strip read after its string changes reads the string it had", async () => {
  const { js, lib } = await compiled("strip-held", `pub fn rest_after<'a>(s: &'a str, t: &'a str) -> &'a str {
    let mut s = s;
    let rest = s.strip_prefix("é");
    s = t;
    match rest {
        Some(rest) => rest,
        None => s,
    }
}
`);
  expect(js).toContain('.startsWith("é")');
  expect([lib.restAfter("éa", "xy"), lib.restAfter("a", "xy")]).toEqual(["a", "xy"]);
});
