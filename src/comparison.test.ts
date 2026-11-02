import { test } from "node:test";
import assert from "node:assert/strict";
import { officialRoleUrl, toggleComparison } from "./comparison.ts";

test("comparison selects at most two distinct roles and supports removal and replacement", () => {
  const first = toggleComparison([], "1");
  const pair = toggleComparison(first, "2");
  assert.deepEqual(first, ["1"]);
  assert.deepEqual(pair, ["1", "2"]);
  assert.deepEqual(toggleComparison(pair, "3"), ["1", "2"]);
  assert.deepEqual(toggleComparison(pair, "1"), ["2"]);
  assert.deepEqual(toggleComparison(toggleComparison(pair, "1"), "3"), [
    "2",
    "3",
  ]);
  assert.deepEqual(toggleComparison(first, "1"), []);
});

test("official role URLs only accept numeric identifiers within the backend limit", () => {
  assert.equal(
    officialRoleUrl("1622610161214590148"),
    "http://cyg.changyou.com/details?goodsCode=1622610161214590148",
  );
  assert.ok(officialRoleUrl("1".repeat(32)));
  for (const value of [
    "",
    "1".repeat(33),
    "1&other=2",
    "../",
    "https://example.com",
    "１２３",
  ])
    assert.equal(officialRoleUrl(value), null);
});
