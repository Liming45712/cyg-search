import type { Role } from './filter';
import { defaultSpirit, qualityRates, spiritStats, treasureStats, type Quality, type SpiritInput } from './spirit';

export interface SpiritSimulation {
  name: string;
  input: SpiritInput;
  missing: string[];
  stale: boolean;
}

export function simulationFromSpirit(item: Role, starGeneral: boolean): SpiritSimulation {
  if (item.goods_type !== 5 || !item.detail) throw new Error('请先拉取该星灵的详情，再进行模拟。');
  const attrs = item.detail.attrs;
  const missing: string[] = [];
  const number = (name: string, max = Infinity, integer = false) => {
    const value = attrs[name]?.num;
    if (typeof value === 'number' && Number.isFinite(value) && value >= 0 && value <= max && (!integer || Number.isInteger(value))) return value;
    missing.push(name);
    return undefined;
  };
  const quality = (name: string): Quality | '' => {
    const value = attrs[name]?.raw ?? '';
    if (Object.hasOwn(qualityRates,value)) return value as Quality;
    missing.push(`${name}品质`);
    return '';
  };
  const input = defaultSpirit();
  input.level = number('等级',159,true);
  input.quality = quality('资质');
  input.growth = quality('成长');
  input.stars = number('强化',10,true);
  input.huixin = number('慧心',139,true) ?? 1;
  input.starGeneral = starGeneral;
  for (const stat of spiritStats) input.aptitude[stat] = number(stat === '体质' ? '体力资质' : `${stat}资质`);
  input.insight = 1;
  input.allocation = input.insightStat = input.fixedStat = input.huixinStat = '无';
  input.divineTreasure = false;
  for (const stat of treasureStats) { input.treasureBase[stat] = undefined; input.treasureExtra[stat] = undefined; }
  for (const stat of spiritStats) input.treasureFive[stat] = undefined;
  return { name: item.name, input, missing: missing.filter(name=>name!=='慧心'), stale: !!item.needs_refresh || !!item.detail_error };
}
