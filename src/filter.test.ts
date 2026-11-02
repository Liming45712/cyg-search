import { test } from 'node:test'
import assert from 'node:assert/strict'
import { applyFilters, defaults, rangeError, type Role } from './filter.ts'

const role = (code: string, price: number | null, score?: number, equipment: Record<string, number> = {}): Role => ({
  code, price, name: `角色${code}`, title: '', job: '剑客', server_id: '104', status: 2,
  listed_at: '2026-11-01 10:00:00', offline_at: '', checked_at: 1, detail_at: null, detail_error: null,
  detail: score === undefined ? null : { attrs: { 装备总评分: { num: score, raw: String(score) } }, equipment },
})
test('inclusive ranges, missing values, equipment AND/OR and descending sort', () => {
  const roles = [role('1', 100, 500, { 至尊武器: 1, 神佑防具: 3 }), role('2', 200, 600, { 至尊武器: 1 }), role('3', null)]
  const f = defaults()
  f.price = [100, 200]; f.minimums = { 装备总评分: 500 }
  assert.deepEqual(applyFilters(roles, f).map(r => r.code), ['1', '2'])
  f.equipment = ['至尊武器', '神佑防具']
  assert.equal(applyFilters(roles, f).length, 1)
  f.matchAll = false
  assert.equal(applyFilters(roles, f).length, 2)
  f.price = [null, null]; f.minimums = {}; f.equipment = []; f.desc = true
  assert.deepEqual(applyFilters(roles, f).map(r => r.code), ['2', '1', '3'])
  f.sort = '装备总评分'
  assert.deepEqual(applyFilters(roles, f).map(r => r.code), ['2', '1', '3'])
  f.price = [601, 600]
  assert.ok(rangeError(f)); assert.equal(applyFilters(roles, f).length, 0)
})
test('server, job, status, keyword and whitespace are respected', () => {
  const f = defaults(); const rows = [role('1', 100), { ...role('2', 200), server_id: '615' }]
  f.keyword = ' 角色1 '
  assert.equal(applyFilters(rows, f).length, 1)
  f.jobs = ['医师']; assert.equal(applyFilters(rows, f).length, 0)
  f.jobs = []; f.status = '1'; assert.equal(applyFilters(rows, f).length, 0)
})

test('Chinese name substring matches the role name', () => {
  const f = defaults()
  const rows = [{ ...role('1', 100), name: '月下听雨123' }, { ...role('2', 200), name: '山间听风' }]
  f.keyword = ' 听雨 '
  assert.deepEqual(applyFilters(rows, f).map(r => r.code), ['1'])
  f.keyword = ''
  assert.equal(applyFilters(rows, f).length, 2)
})

test('equipment selection matches presence regardless of the number of pieces', () => {
  const f = defaults()
  f.equipment = ['神佑防具']
  const rows = [role('1', 100, 500, { 神佑防具: 1 }), role('2', 200, 500, { 神佑防具: 5 }), role('3', 300, 500), role('4', 400)]
  assert.deepEqual(applyFilters(rows, f).map(r => r.code), ['1', '2'])
})

test('attribute minimum is inclusive, has no upper bound and excludes missing values', () => {
  const f = defaults()
  const rows = [role('1', 100, 499), role('2', 200, 500), role('3', 300, 90000), role('4', 400)]
  f.minimums = { 装备总评分: 500 }
  assert.deepEqual(applyFilters(rows, f).map(r => r.code), ['2', '3'])
  f.minimums.装备总评分 = 0
  assert.deepEqual(applyFilters(rows, f).map(r => r.code), ['1', '2', '3'])
  f.minimums.装备总评分 = null
  assert.equal(applyFilters(rows, f).length, 4)
  f.minimums.装备总评分 = NaN
  assert.ok(rangeError(f))
})
