// What TypeScript's library doesn't say of a builtin that its Rust binding
// needs, each entry the fact it lacks, its owner and the test that proves
// it (#1904). Never a member or a signature: those are the library's, which
// `builtins/library.ts` reads. An entry may name, or say what throws; it
// may not narrow a declared union, an optional key or an overload.

/** A Rust name for what the library leaves unnamed, or names otherwise. */
export type Named = { rust: string; why: string; proof: string };

/** Each by its library name: an alias, an inline union `Owner.member`'s,
 * or a parameter's union, `new Owner(param)`'s. */
export const NAMES: Record<string, Named> = {
  "Intl.LocaleCollationCaseFirst": {
    rust: "CaseFirst",
    why: "the established Rust name of `caseFirst`'s values, which the collator's options share; its members are the alias's",
    proof: "test/intl-locale.test.ts -t \"a locale's options and its parts\"",
  },
  "Intl.LocaleHourCycleKey": {
    rust: "HourCycle",
    why: "the established Rust name of `hourCycle`'s values, which the date formatter's options share; its members are the alias's",
    proof: "test/intl-locale.test.ts -t \"a locale's options and its parts\"",
  },
  "Intl.TextInfo.direction": {
    rust: "TextDirection",
    why: "an inline union of `\"ltr\" | \"rtl\"`, which TypeScript names nothing",
    proof: "test/intl-locale.test.ts -t \"what a locale knows of its text, its week and its time zones\"",
  },
  "new Intl.Locale(tag)": {
    rust: "IntoLocaleTag",
    why: "the sealed input of `string | Intl.Locale`, which TypeScript names nothing; not `IntoLocalesArgument`, whose lists a tag isn't",
    proof: "test/intl-locale.test.ts -t \"a locale of a tag or of another locale\"",
  },
};

/** What throws, which a TypeScript signature doesn't say: its error. */
export type Throws = { error: "RangeError" | "TypeError"; why: string; proof: string };

/** Each by its call's JS, `new Intl.Locale`. */
export const THROWS: Record<string, Throws> = {
  "new Intl.Locale": {
    error: "RangeError",
    why: "ECMA-402's `Intl.Locale` throws a `RangeError` of a tag that isn't a well-formed language tag",
    proof: "test/intl-locale.test.ts -t \"a locale of a tag or of another locale\"",
  },
};

/** What a builtin runs of the program's code, which TypeScript's library
 * doesn't say: none, `runs_nothing` (ADR 0867), or only what it's given,
 * `runs_only_given` (ADR 0898), whose arguments the borrow analysis checks
 * as JS reads them. A `RefCell` borrowed across its call isn't counted
 * (#2082). Each by its JS, `new Intl.Locale` or `Intl.Locale.prototype.x`,
 * with the ECMA-402 steps that show it, of builtins as the host has them. */
export type Runs = { attribute: "runs_nothing" | "runs_only_given"; spec: string; proof: string };

const formatterRuns = (name: string, create: string): Record<string, Runs> => ({
  [`new Intl.${name}`]: {
    attribute: "runs_only_given",
    spec: `ECMA-402 ${create}: CanonicalizeLocaleList(locales), which reads a string or a Locale's [[Locale]], and GetOption of options' properties, what it's given`,
    proof: "test/builtins-runs-nothing.test.ts",
  },
  [`Intl.${name}.prototype.resolvedOptions`]: {
    attribute: "runs_nothing",
    spec: `ECMA-402 Intl.${name}.prototype.resolvedOptions: reads its own internal slots`,
    proof: "test/builtins-runs-nothing.test.ts",
  },
});

const localeReads = (method: string, spec: string): Record<string, Runs> => ({
  [`Intl.Locale.prototype.${method}`]: { attribute: "runs_nothing", spec, proof: "test/builtins-runs-nothing.test.ts" },
});

export const RUNS: Record<string, Runs> = {
  "new Intl.Locale": {
    attribute: "runs_only_given",
    spec: "ECMA-402 Intl.Locale ( tag [ , options ] ): ToString of a tag that isn't an Intl.Locale, else its [[Locale]], and GetOption of options' properties, what it's given",
    proof: "test/builtins-runs-nothing.test.ts",
  },
  ...localeReads("maximize", "ECMA-402 Intl.Locale.prototype.maximize: Add Likely Subtags of its [[Locale]]"),
  ...localeReads("minimize", "ECMA-402 Intl.Locale.prototype.minimize: Remove Likely Subtags of its [[Locale]]"),
  ...localeReads("toString", "ECMA-402 Intl.Locale.prototype.toString: its [[Locale]]"),
  ...localeReads("getCalendars", "Intl Locale Info: CalendarsOfLocale of its [[Locale]]"),
  ...localeReads("getCollations", "Intl Locale Info: CollationsOfLocale of its [[Locale]]"),
  ...localeReads("getHourCycles", "Intl Locale Info: HourCyclesOfLocale of its [[Locale]]"),
  ...localeReads("getNumberingSystems", "Intl Locale Info: NumberingSystemsOfLocale of its [[Locale]]"),
  ...localeReads("getTimeZones", "Intl Locale Info: TimeZonesOfLocale of its [[Locale]]"),
  ...localeReads("getTextInfo", "Intl Locale Info: TextInfoOfLocale of its [[Locale]]"),
  ...localeReads("getWeekInfo", "Intl Locale Info: WeekInfoOfLocale of its [[Locale]]"),
  ...formatterRuns("Collator", "InitializeCollator"),
  ...formatterRuns("NumberFormat", "InitializeNumberFormat"),
  ...formatterRuns("DateTimeFormat", "CreateDateTimeFormat"),
  ...formatterRuns("PluralRules", "InitializePluralRules"),
  ...formatterRuns("RelativeTimeFormat", "InitializeRelativeTimeFormat"),
  ...formatterRuns("ListFormat", "Intl.ListFormat ( [ locales [ , options ] ] )"),
};

/** `js`'s attribute of what it runs, `RUNS`', none of one not there. */
export const runs = (js: string): string[] => (RUNS[js] ? [RUNS[js].attribute] : []);

