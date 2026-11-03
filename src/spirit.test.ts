import { test } from 'node:test';
import assert from 'node:assert/strict';
import { calculateSpirit, defaultSpirit, spiritStats } from './spirit.ts';

test('matches all eight saved Excel example outputs without intermediate rounding', () => {
  const result = calculateSpirit(defaultSpirit());
  assert.equal(result.error, null);
  if (result.error !== null) return;
  assert.deepEqual(result.total, { 力量: 963, 体质: 168, 心力: 168, 耐力: 168, 敏捷: 168 });
  const expected = [294.9408, 102.06504, 196.50015, 35.44464, 23.99628, 196.50015, 86.633778, 34.2804];
  result.rows.forEach((row, i) => assert.ok(Math.abs(row.bonus - expected[i]) < 1e-9, row.name));
});

test('preserves strict Excel Huixin thresholds and the divine treasure gate', () => {
  for (const [h, fixed, assigned] of [[0, 0, 0], [36, 18, 18], [37, 37, 37], [72, 72, 72], [73, 146, 73], [108, 216, 108], [109, 327, 218], [139, 417, 278]]) {
    const input = defaultSpirit();
    input.huixin = h;
    input.divineTreasure = true;
    input.fixedStat = '体质';
    input.huixinStat = '敏捷';
    input.insightStat = '心力';
    input.insight = 10;
    const result = calculateSpirit(input);
    assert.equal(result.error, null);
    if (result.error !== null) continue;
    assert.equal(result.fixed, fixed);
    assert.equal(result.assigned, assigned);
    assert.equal(result.total.体质, 168 + fixed);
    assert.equal(result.total.敏捷, 168 + assigned);
    assert.equal(result.total.心力, 178);
    input.divineTreasure = false;
    assert.equal(calculateSpirit(input).fixed, 0);
  }
});

test('treasure five stats and direct attributes feed the correct outputs, including cross-stat spirit', () => {
  const input = defaultSpirit();
  input.level = 0;
  input.starGeneral = false;
  input.quality = '逆天';
  input.growth = '逆天';
  input.stars = 10;
  input.treasureFive = { 力量: 10, 体质: 20, 心力: 30, 耐力: 40, 敏捷: 50 };
  input.aptitude = { 力量: 2, 体质: 3, 心力: 4, 耐力: 5, 敏捷: 6 };
  input.treasureBase = { HP: 100, SP: 20, 攻击: 30, 防御: 40, 精神: 50, 闪避: 60 };
  input.treasureExtra = { HP: 1, SP: 2, 攻击: 3, 防御: 4, 精神: 5, 闪避: 6 };
  const result = calculateSpirit(input);
  assert.equal(result.error, null);
  if (result.error !== null) return;
  assert.deepEqual(result.rows.map(row => row.panel), [461, 382, 53, 244, 210, 20, 231, 366]);
  result.rows.forEach(row => assert.ok(Math.abs(row.bonus - row.panel * 0.15) < 1e-9));
});

test('no spirit with zero optional inputs gives zero, blanks in treasure count as zero', () => {
  const input = defaultSpirit();
  input.level = 0;
  for (const stat of spiritStats) input.treasureFive[stat] = undefined;
  input.treasureBase.HP = undefined;
  const result = calculateSpirit(input);
  assert.equal(result.error, null);
  if (result.error !== null) return;
  assert.ok(result.rows.every(row => row.panel === 0 && row.bonus === 0));
});

test('the original no-allocation option leaves generated points unassigned', () => {
  const input = defaultSpirit();
  input.allocation = '无';
  input.insightStat = '无';
  input.fixedStat = '无';
  input.huixinStat = '无';
  input.huixin = 109;
  input.insight = 100;
  input.divineTreasure = true;
  const result = calculateSpirit(input);
  assert.equal(result.error, null);
  if (result.error !== null) return;
  assert.ok(Object.values(result.total).every(value => value === 168));
});

test('rejects missing required values, invalid numbers, unsupported levels and overflow', () => {
  for (const invalid of [undefined, NaN, Infinity, -1]) {
    const input = defaultSpirit();
    input.aptitude.力量 = invalid;
    assert.ok(calculateSpirit(input).error);
    const huixin = defaultSpirit();
    huixin.huixin = invalid;
    assert.ok(calculateSpirit(huixin).error);
  }
  for (const h of [140, 1.5]) {
    const input = defaultSpirit(); input.huixin = h;
    assert.ok(calculateSpirit(input).error);
  }
  const input = defaultSpirit();
  for (const level of [undefined,NaN,-1,160,1.5]) { input.level = level; assert.ok(calculateSpirit(input).error); }
  input.level = 159;
  input.treasureExtra.HP = -1;
  assert.ok(calculateSpirit(input).error);
  input.treasureExtra.HP = 0;
  input.aptitude.力量 = Number.MAX_VALUE;
  assert.ok(calculateSpirit(input).error);
});


test('supports actual marketplace levels without rounding to Excel milestones', () => {
  for (const level of [1,50,100,157]) {
    const input = defaultSpirit(); input.level = level;
    const result = calculateSpirit(input); assert.equal(result.error,null);
    if(result.error!==null)continue;
    assert.equal(result.base,level+9); assert.equal(result.points,level*5);
  }
});
