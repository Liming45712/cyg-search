import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ref } from 'vue';
import { useScrollRows } from './scroll-rows.ts';

test('appends local results in order, coalesces repeated bottom events and stops at the end', async () => {
  const rows = ref(Array.from({ length: 125 }, (_, i) => i));
  const list = useScrollRows(rows);
  assert.deepEqual(list.visibleRows.value, rows.value.slice(0, 50));
  await Promise.all([list.loadMore(), list.loadMore(), list.loadMore()]);
  assert.deepEqual(list.visibleRows.value, rows.value.slice(0, 100));
  await list.loadMore();
  assert.deepEqual(list.visibleRows.value, rows.value);
  assert.equal(list.hasMore.value, false);
  await list.loadMore();
  assert.equal(list.visibleRows.value.length, 125);
});

test('new filters reset the loaded batch, empty results stay empty and cache updates retain the loaded limit', async () => {
  const rows = ref(Array.from({ length: 120 }, (_, i) => i));
  const list = useScrollRows(rows);
  await list.loadMore();
  rows.value = [...rows.value, 120];
  assert.equal(list.visibleRows.value.length, 100);
  list.resetRows();
  assert.equal(list.visibleRows.value.length, 50);
  rows.value = [];
  assert.deepEqual(list.visibleRows.value, []);
  assert.equal(list.hasMore.value, false);
  await list.loadMore();
  rows.value = [1, 2, 3];
  assert.deepEqual(list.visibleRows.value, [1, 2, 3]);
  assert.equal(list.hasMore.value, false);
});
