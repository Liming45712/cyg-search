import { test } from 'node:test';
import assert from 'node:assert/strict';
import { equipmentAttribute, equipmentMinimum, marketEquipment } from './equipment';
import type { Role } from './filter';

const role = (stats: [string, number, string?][]): Role => ({
  goods_type: 3, code: '123', name: '装备', title: '', job: '', price: null,
  server_id: '104', status: 1, listed_at: '2026-11-04 12:00:00', offline_at: '', checked_at: 0,
  detail_at: 1, detail_error: null, detail: { attrs: {}, equipment: {}, gear: {
    kind: '武器', level: 150, score: 1000, stats, effects: ['宝石效果：攻击+10'], set: [], raw: ['基本攻击力 +40'],
  } },
});
test('basic attack, spirit and defense are normalized independently without summing repeated rows', () => {
  const source = role([['基本攻击力',40],['攻击力',40],['基本精神力',50],['基本防御力',60]]);
  const item = marketEquipment(source);
  assert.deepEqual(item.stats.map(s=>s[0]), ['攻击力','攻击力','精神力','防御力']);
  assert(equipmentMinimum(item,'攻击力',40));
  assert(!equipmentMinimum(item,'攻击力',80));
  assert(equipmentMinimum(item,'精神力',50));
  assert(!equipmentMinimum(item,'防御力',61));
  assert(!equipmentMinimum(item,'暴击率',1));
  assert.equal(item.price,null);
  assert.equal(source.detail?.gear?.stats[0][0],'基本攻击力');
  assert.equal(equipmentAttribute('基本命中率'),'基本命中率');
});
test('missing equipment detail remains pending and never satisfies a numeric minimum', () => {
  const source = role([]); source.detail = null; source.detail_error = '超时';
  const item = marketEquipment(source);
  assert(!item.ready); assert.equal(item.error,'超时');
  assert(!equipmentMinimum(item,'攻击力',0));
  assert(equipmentMinimum(item,'攻击力',undefined));
});
