// An excerpt of vercel-labs/ui@220c1a0, packages/core/src/foundation/kernel/
// details.ts (Apache-2.0, Copyright Vercel, Inc.): its types and
// createDetails as upstream writes them, of five of its reasons, the
// reference test/create-details.test.ts holds the compiled one to.

/**
 * How the user caused a change.
 * @stable
 */
type ChangeSource =
  "mouse" | "touch" | "pen" | "keyboard" | "virtual" | "programmatic" | "agent"

/**
 * Every reason a change can report, what it means, and the DOM event behind
 * it. Components report a subset, exported as a union per callback, like
 * `PopoverOpenChangeReason`. Checking `details.reason` narrows `details.event`
 * to the type listed here. Controllers and cascades have no event, so
 * `event` is always optional.
 * @stable
 */
type ReasonVocabulary = {
  press: MouseEvent | KeyboardEvent
  "escape-key": KeyboardEvent
  "input-commit": FocusEvent | KeyboardEvent
  drop: DragEvent
  reset: undefined
}

/**
 * A reason from the shared vocabulary.
 * @stable
 */
type ChangeReason = keyof ReasonVocabulary

/** The DOM event a reason reports, from the vocabulary. Any `Event` for reasons outside it. */
type EventOf<Reason extends string> = Reason extends ChangeReason
  ? ReasonVocabulary[Reason]
  : Event

/**
 * What every `on<Key>Change` callback receives after the new value. One
 * member per reason, so checking `reason` narrows `event`.
 * @stable
 */
type ChangeDetails<Reason extends string = string> = Reason extends unknown
  ? {
      /** Why it changed, like `"escape-key"`. */
      reason: Reason
      /** The DOM event behind it, typed by `reason`. Undefined for controllers and cascades. */
      event?: EventOf<Reason>
      /** How the user caused it: `"mouse"`, `"touch"`, `"pen"`, `"keyboard"`, `"virtual"` (a click with no pointer or key behind it, which may be assistive technology), `"agent"` (an AI agent's call through `@ui-sdk/core/agent`) or `"programmatic"`. For analytics and focus choices, never for behavior. */
      source: ChangeSource
      /** Stops the change. A controlled component ignores it, since the next value comes from you. */
      cancel: () => void
      /** Whether `cancel()` was called. */
      readonly canceled: boolean
    }
  : never

/**
 * What an `on<Value>Commit` callback receives after the value: why and how the
 * interaction that ended changed it. Commits can't be canceled.
 * @stable
 */
type CommitDetails<Reason extends string = string> = Reason extends unknown
  ? {
      /** What changed the value, like `"drag"` or `"step-key"`. */
      reason: Reason
      /** The DOM event that ended the interaction, or the one behind the last change when the end was another kind, like a blur after a wheel. Undefined for controllers. */
      event?: EventOf<Reason>
      /** How the user caused it, like `"mouse"` or `"keyboard"`. */
      source: ChangeSource
    }
  : never

/**
 * The details a callback receives: the `reason`, the `event` and anything
 * else `given`, like which item changed, with how the user caused it. A
 * change's details cancel it, through `cancel()` and `canceled`. A commit's,
 * with `cancelable` false, leave those out, since commits can't be canceled.
 */
function createDetails<
  const Given extends { reason: string; event?: Event },
  Cancelable extends boolean = true,
>(
  given: Given,
  source: ChangeSource,
  cancelable: Cancelable = true as Cancelable
) {
  type Kind = Cancelable extends true
    ? ChangeDetails<Given["reason"]>
    : CommitDetails<Given["reason"]>
  type Details = Kind & Omit<Given, "reason" | "event" | "source">
  // One object for every reason. The per-reason union only narrows for readers,
  // so it can't be built from a generic reason, and the cast happens once, here.
  if (!cancelable) {
    return { ...given, source } as unknown as Details
  }
  let canceled = false
  return {
    ...given,
    source,
    cancel: () => {
      canceled = true
    },
    get canceled() {
      return canceled
    },
  } as unknown as Details
}

export {
  type ChangeDetails,
  type ChangeReason,
  type ChangeSource,
  type CommitDetails,
  createDetails,
  type ReasonVocabulary,
}
