// `Intl.Locale`, written from TypeScript's own library (#1904): Rust callers
// of each member the library declares, run on the host, its strict
// TypeScript callers against the declarations rust-js writes of them, and
// the generator's refusal of a form it doesn't bind. builtins/policy.ts
// cites these tests by name, each the proof of an entry.

import { beforeAll, expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Refused, type Library, writeClass, writing } from "../builtins/library";
import { NAMES, THROWS } from "../builtins/policy";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `rust` compiled against the js crate, imported, and its declarations. */
async function compiled(name: string, rust: string): Promise<{ lib: any; js: string; dir: string }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  return { lib: await import(join(dir, "lib.js")), js: readFileSync(join(dir, "lib.js"), "utf8"), dir };
}

// `new Intl.Locale(tag, options)`'s tag is `string | Intl.Locale`, the
// policy's `IntoLocaleTag`: text, owned or borrowed, or another locale, as it
// is. A tag that isn't one is the `RangeError` the policy says it throws.
test("a locale of a tag or of another locale", async () => {
  const { lib, js } = await compiled(
    "intl-locale-tags",
    `use js::intl::{locale, LocaleOptions};
pub fn tags() -> (String, String, String, String, String) {
    let us = locale::new("en-US", None).unwrap();
    let owned = locale::new(String::from("de-DE"), None).unwrap().to_string();
    let borrowed = locale::new(&String::from("fr-CA"), None).unwrap().to_string();
    let of_another = locale::new(us, Some(&LocaleOptions { region: Some("GB"), ..Default::default() })).unwrap().to_string();
    let refused = locale::new("not a tag!", None).err().map(|e| e.name()).unwrap_or_default();
    (us.to_string(), owned, borrowed, of_another, refused)
}
`,
  );
  expect(js).toContain('new Intl.Locale(us, { region: "GB" })');
  expect(lib.tags()).toEqual(["en-US", "de-DE", "fr-CA", "en-GB", "RangeError"]);
  expect(NAMES["new Intl.Locale(tag)"].rust).toBe("IntoLocaleTag");
  expect(THROWS["new Intl.Locale"].error).toBe("RangeError");
});

// Its options, each `None` not given, and the parts it reads back: one it
// wasn't given is `undefined`, a `None` (`none_undefined`), but `numeric`,
// which JS gives `false`; `CaseFirst` and `HourCycle` the policy's names of
// the library's unions.
test("a locale's options and its parts", async () => {
  const { lib } = await compiled(
    "intl-locale-parts",
    `use js::intl::{locale, CaseFirst, HourCycle, LocaleOptions};
pub fn parts() -> (String, Option<String>, Option<String>, Option<bool>, Option<bool>, Option<HourCycle>, Option<HourCycle>, Option<CaseFirst>, Option<String>) {
    let en = locale::new("en", None).unwrap();
    let set = locale::new("en-Latn-US", Some(&LocaleOptions {
        hour_cycle: Some(HourCycle::H23),
        numeric: Some(true),
        case_first: Some(CaseFirst::Upper),
        calendar: Some("gregory"),
        ..Default::default()
    })).unwrap();
    (set.base_name(), set.script(), en.region(), en.numeric(), set.numeric(), en.hour_cycle(), set.hour_cycle(), set.case_first(), set.calendar())
}
`,
  );
  expect(lib.parts()).toEqual(["en-Latn-US", "Latn", undefined, false, true, undefined, "h23", "upper", "gregory"]);
});

// What a locale knows that TypeScript's ESNext library declares: its text's
// direction, the policy's `TextDirection`; its week, `firstDay` and
// `weekend` alone, as the library has them; its time zones, `undefined` of
// a locale with no region; its calendars and hour cycles.
test("what a locale knows of its text, its week and its time zones", async () => {
  const { lib } = await compiled(
    "intl-locale-known",
    `use js::intl::{locale, TextDirection, WeekInfo};
pub fn known() -> (WeekInfo, WeekInfo, Option<TextDirection>, Option<TextDirection>, Option<usize>, Option<usize>, bool, bool) {
    let us = locale::new("en-US", None).unwrap();
    let de = locale::new("de-DE", None).unwrap();
    let ar = locale::new("ar", None).unwrap();
    let en = locale::new("en", None).unwrap();
    (
        us.get_week_info(),
        de.get_week_info(),
        us.get_text_info().direction,
        ar.get_text_info().direction,
        us.get_time_zones().map(|zones| zones.len()),
        en.get_time_zones().map(|zones| zones.len()),
        us.get_calendars().contains(&String::from("gregory")),
        !us.get_hour_cycles().is_empty(),
    )
}
`,
  );
  const [us, de, ...rest] = lib.known();
  expect([us.firstDay, us.weekend, de.firstDay, de.weekend]).toEqual([7, [6, 7], 1, [6, 7]]);
  expect(Object.keys(us)).toEqual(["firstDay", "weekend"]);
  expect(rest).toEqual(["ltr", "rtl", expect.any(Number), undefined, true, true]);
  expect(NAMES["Intl.TextInfo.direction"].rust).toBe("TextDirection");
});

// A host that hasn't ESNext's `getWeekInfo`, an older browser's, is told by
// `js::is_function((locale, Locale::get_week_info))`, `typeof
// locale.getWeekInfo === "function"`: nothing called to find out, no
// `TypeError` caught, the locale itself the receiver (#1831).
test("a locale's week info where the host has it, and none where it hasn't", async () => {
  const { lib, js } = await compiled(
    "intl-locale-week-detected",
    `use js::intl::{locale, Locale};
pub fn first_day(tag: &str) -> Option<f64> {
    let locale = locale::new(tag, None).ok()?;
    if js::is_function((locale, Locale::get_week_info)) {
        return Some(locale.get_week_info().first_day);
    }
    None
}
`,
  );
  expect(js).toContain('if (typeof locale.getWeekInfo === "function") {\n    return locale.getWeekInfo().firstDay;');
  expect([lib.firstDay("en-US"), lib.firstDay("de-DE")]).toEqual([7, 1]);
  const own = Object.getOwnPropertyDescriptor(Intl.Locale.prototype, "getWeekInfo")!;
  delete (Intl.Locale.prototype as any).getWeekInfo;
  try {
    expect(lib.firstDay("en-US")).toBeUndefined();
  } finally {
    Object.defineProperty(Intl.Locale.prototype, "getWeekInfo", own);
  }
});

// A strict TypeScript caller of what rust-js declares of them: the
// library's own types, `Intl.Locale` and `Intl.WeekInfo`, given and read.
test("a strict TypeScript caller of the generated locale", async () => {
  const { dir } = await compiled(
    "intl-locale-typescript",
    `use js::intl::{locale, Locale, WeekInfo};
pub fn tagged(tag: &str) -> &'static Locale {
    locale::new(tag, None).unwrap()
}
pub fn week(of: &Locale) -> WeekInfo {
    of.get_week_info()
}
`,
  );
  expect(existsSync(join(dir, "lib.d.ts"))).toBe(true);
  writeFileSync(
    join(dir, "use.ts"),
    `import { tagged, week } from "./lib.js";
const locale: Intl.Locale = tagged("en-US");
const info: Intl.WeekInfo = week(locale);
export const first: number = info.firstDay;
export const weekend: number[] = info.weekend;
export const wrong: string = week(locale).firstDay;
`,
  );
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false, allowJs: true, lib: ["esnext"], typeRoots: [join(root, "node_modules/@types")] },
      files: ["use.ts"],
    }),
  );
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors).toHaveLength(1);
  expect(errors[0]).toContain("use.ts(6,");
});

// What the generator doesn't bind it refuses by the member's name and its
// file, never written as `any` or narrowed: a made-up class whose method
// gives `any`.
test("the generator refuses a form it doesn't bind, by its name", () => {
  const file = "/lib.made-up.d.ts";
  const library: Library = new Map([
    ["Made.Thing", [
      { file, decl: { kind: "const", name: "Thing", type: { kind: "object", members: [{ kind: "construct", typeParameters: [], params: [], returns: { kind: "reference", name: "Thing", args: [] } }] } } },
      { file, decl: { kind: "interface", name: "Thing", extends: [], members: [{ kind: "method", name: "loose", params: [], returns: { kind: "keyword", keyword: "any" } }] } },
    ]],
  ]);
  expect(() => writeClass(writing(library), "Made.Thing", "thing")).toThrow(Refused);
  expect(() => writeClass(writing(library), "Made.Thing", "thing")).toThrow(/^builtins\/library\.ts: Made\.Thing\.loose \(lib\.made-up\.d\.ts\): a result type .*"any".* never `any`/);
});
