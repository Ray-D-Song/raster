import diagnostics, {
  channel,
  subscribe,
  unsubscribe,
  hasSubscribers,
} from "node:diagnostics_channel";
import legacy from "diagnostics_channel";
import Module from "node:module";

it("registers diagnostics_channel as a builtin module", () => {
  expect(Module.isBuiltin("diagnostics_channel")).toBe(true);
  expect(require("diagnostics_channel")).toBe(diagnostics);
  expect(require("node:diagnostics_channel")).toBe(diagnostics);
  expect(legacy).toBe(diagnostics);
});

it("returns singleton channels and publishes in order", () => {
  const first = channel("test-channel");
  const second = channel("test-channel");
  expect(first).toBe(second);

  const seen: unknown[] = [];
  first.subscribe((message) => {
    seen.push(message);
  });
  first.publish("one");
  first.publish("two");
  expect(seen).toEqual(["one", "two"]);
  expect(first.hasSubscribers).toBe(true);
  expect(hasSubscribers("test-channel")).toBe(true);
});

it("supports subscribe helpers and unsubscribe", () => {
  const fn = () => {};
  subscribe("helper-channel", fn);
  expect(hasSubscribers("helper-channel")).toBe(true);
  expect(unsubscribe("helper-channel", fn)).toBe(true);
  expect(unsubscribe("missing-channel", fn)).toBe(false);
});

it("continues notifying subscribers when one unsubscribes during publish", () => {
  const ch = channel("unsubscribe-during-publish");
  const seen: string[] = [];
  const a = () => {
    seen.push("a");
    ch.unsubscribe(a);
  };
  const b = () => {
    seen.push("b");
  };
  ch.subscribe(a);
  ch.subscribe(b);
  ch.publish("payload");
  expect(seen).toEqual(["a", "b"]);
});
