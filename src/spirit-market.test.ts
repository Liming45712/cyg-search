import { test } from 'node:test';
import assert from 'node:assert/strict';
import { marketSpirit, spiritPanelFields } from './spirit-market.ts';
import { applyFilters, defaults, type Attr, type Role } from './filter.ts';
const attr = (raw: string, num: number | null = null): Attr => ({ raw, num });
function sample(): Role {
  return { code: '1', name: '蜗牛', title: '159级 神工 神工', job: '', price: 1208, server_id: '104',
    status: 2, listed_at: '', offline_at: '', checked_at: 0, detail_at: 0, detail_error: null, goods_type: 5,
    detail: { attrs: { 资质: attr('神工'), 成长: attr('神工'), 强化: attr('10', 10),
      生命值: attr('3003', 3003), 技能值: attr('1335', 1335), 攻击力: attr('420', 420), 防御力: attr('504', 504),
      精神力: attr('712', 712), 灵巧度: attr('2726', 2726), 命中率: attr('420', 420), 闪避率: attr('3894', 3894) }, equipment: {} } };
}
test('official panel is multiplied once, including existing Huixin or treasure effects', () => {
  const item = sample();
  const result = marketSpirit(item, true);
  const expected = [363.363, 161.535, 50.82, 60.984, 86.152, 329.846, 50.82, 471.174];
  spiritPanelFields.forEach((field, i) => assert.ok(Math.abs(result.detail!.attrs[`加成${field.name}`].num! - expected[i]) < 1e-9));
  assert.equal(marketSpirit(item, false).detail!.attrs.加成攻击.num, 46.2);
  assert.equal(item.detail!.attrs.加成攻击, undefined);
});
test('missing values stay missing and cannot pass minimum filters', () => {
  const item = sample();
  item.list_attrs = item.detail!.attrs;
  item.detail = null;
  const result = marketSpirit(item, true);
  assert.equal(result.detail!.attrs.加成攻击.num, null);
  const filters = defaults('104'); filters.minimums.加成攻击 = 0;
  assert.equal(applyFilters([result], filters).length, 0);
  const invalid = sample(); invalid.detail!.attrs.强化.num = 11;
  assert.equal(marketSpirit(invalid, true).detail!.attrs.加成HP.num, null);
  invalid.detail!.attrs.强化.num = 10;
  invalid.detail!.attrs.成长.raw = '未知';
  assert.equal(marketSpirit(invalid, true).detail!.attrs.加成HP.num, null);
});
test('stale list quality does not mix with an older cached detail panel', () => {
  const item = sample();
  item.list_attrs = { 资质: attr('逆天'), 成长: attr('逆天'), 强化: attr('0', 0) };
  assert.equal(marketSpirit(item, false).detail!.attrs.加成攻击.num, 46.2);
});

test('all eight bonus columns sort the full result numerically with missing values last', () => {
  const rows = Array.from({ length: 60 }, (_, i) => {
    const row = sample(); row.code = String(i);
    for (const field of spiritPanelFields) row.detail!.attrs[field.source] = attr(String(i), i);
    return marketSpirit(row, true);
  });
  const missing = sample(); missing.code = 'missing'; missing.detail = null;
  rows.push(marketSpirit(missing, true));
  for (const field of spiritPanelFields) {
    const filters = defaults('104'); filters.sort = `加成${field.name}`;
    assert.equal(applyFilters(rows, filters)[0].code, '0');
    filters.desc = true;
    const sorted = applyFilters(rows, filters);
    assert.equal(sorted[0].code, '59');
    assert.equal(sorted.at(-1)!.code, 'missing');
    assert.equal(sorted.slice(0, 50).at(-1)!.code, '10');
  }
});
