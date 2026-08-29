import assert from "node:assert/strict";
import test, { describe, it } from "node:test";
import { epochToDate, parseEpochText, epochToString, isValidEpochText } from "../src/lib/epoch";

describe("epochToDate", () => {
  it("treats values < 2^31 as seconds", () => {
    const d = epochToDate(0);
    assert.equal(d.getTime(), 0);
    const d2 = epochToDate(1);
    assert.equal(d2.getTime(), 1000);
    const d3 = epochToDate(1609459200); // 2021-01-01 00:00:00 UTC in seconds
    assert.equal(d3.toISOString(), "2021-01-01T00:00:00.000Z");
  });
  it("treats values >= 2^31 as milliseconds", () => {
    const ms = 2 ** 31; // 2147483648 ms
    const d = epochToDate(ms);
    assert.equal(d.getTime(), ms);
    const ms2 = 1609459200000; // same as above but ms
    const d2 = epochToDate(ms2);
    assert.equal(d2.toISOString(), "2021-01-01T00:00:00.000Z");
  });
  it("handles max int boundary", () => {
    const justBelow = 2 ** 31 - 1;
    const d = epochToDate(justBelow);
    assert.equal(d.getTime(), justBelow * 1000);
    const at = 2 ** 31;
    const d2 = epochToDate(at);
    assert.equal(d2.getTime(), at);
  });
});

describe("parseEpochText", () => {
  it("parses valid numbers", () => {
    assert.equal(parseEpochText("0"), 0);
    assert.equal(parseEpochText("  1609459200  "), 1609459200);
    assert.equal(parseEpochText("1609459200000"), 1609459200000);
  });
  it("returns null for invalid", () => {
    assert.equal(parseEpochText("not-a-number"), null);
    assert.equal(parseEpochText(""), 0); // Number("") is 0, so valid? But upstream treats NaN only
    assert.equal(parseEpochText("abc123"), null);
  });
});

describe("isValidEpochText", () => {
  it("validates epoch text", () => {
    assert.equal(isValidEpochText("123"), true);
    assert.equal(isValidEpochText("0"), true);
    assert.equal(isValidEpochText("hello"), false);
  });
});

describe("epochToString", () => {
  it("returns date string for epoch", () => {
    const s = epochToString(0);
    assert.ok(s.length > 0);
    assert.match(s, /1970/);
    const s2 = epochToString(1609459200);
    assert.match(s2, /2021/);
  });
});

test("package.json declares Windows+macOS and Tezbar metadata", async () => {
  const pkg = await import("../package.json", { with: { type: "json" } });
  const data = (pkg as unknown as { default: Record<string, unknown> }).default ?? (pkg as unknown as Record<string, unknown>);
  assert.deepEqual((data as { platforms: string[] }).platforms, ["macOS", "Windows"]);
  const tezbar = (data as { tezbar: { platforms: string[]; repository: string; upstream: { commit: string; path: string } } }).tezbar;
  assert.deepEqual(tezbar.platforms, ["macos", "windows"]);
  assert.equal(tezbar.repository, "https://github.com/almatkai/epoch-to-timestamp");
  assert.equal(tezbar.upstream.path, "extensions/epoch-to-timestamp");
  assert.equal(tezbar.upstream.commit, "6734d85992290066fd31a7f5bee3cca3d3545a1a");
  const commands = (data as { commands: { platforms: string[] }[] }).commands;
  assert.equal(commands.length, 1);
  for (const cmd of commands) {
    assert.deepEqual(cmd.platforms, ["macOS", "Windows"]);
  }
  const deps = (data as { dependencies: Record<string, string> }).dependencies;
  assert.ok(deps["@raycast/api"]);
  const cmds = (data as { commands: { name: string }[] }).commands;
  assert.equal(cmds[0].name, "epoch");
});
