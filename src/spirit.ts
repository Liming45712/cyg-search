export const spiritStats = ['力量', '体质', '心力', '耐力', '敏捷'] as const;
export type SpiritStat = typeof spiritStats[number];
export const spiritDirections = ['无', ...spiritStats] as const;
type SpiritDirection = typeof spiritDirections[number];
export const spiritLevels = [0, 83, 108, 133, 148, 158, 159] as const;
export const qualityRates = { 普通: 0.005, 优良: 0.01, 精致: 0.015, 传说: 0.02, 神工: 0.03, 逆天: 0.05 };
export type Quality = keyof typeof qualityRates;
export function spiritRatio(quality: string, growth: string, stars: number | null | undefined): number | null {
  if (!Object.hasOwn(qualityRates, quality) || !Object.hasOwn(qualityRates, growth)
    || typeof stars !== 'number' || !Number.isInteger(stars) || stars < 0 || stars > 10) return null;
  return qualityRates[quality as Quality] + qualityRates[growth as Quality] + stars * 0.005;
}
export const treasureStats = ['HP', 'SP', '攻击', '防御', '精神', '闪避'] as const;
type TreasureStat = typeof treasureStats[number];
type Numbers<K extends string> = Record<K, number | undefined>;
export interface SpiritInput {
  level: number;
  quality: Quality;
  growth: Quality;
  stars: number;
  huixin: number | undefined;
  insight: number | undefined;
  allocation: SpiritDirection;
  insightStat: SpiritDirection;
  fixedStat: SpiritDirection;
  huixinStat: SpiritDirection;
  divineTreasure: boolean;
  starGeneral: boolean;
  aptitude: Numbers<SpiritStat>;
  treasureBase: Numbers<TreasureStat>;
  treasureExtra: Numbers<TreasureStat>;
  treasureFive: Numbers<SpiritStat>;
}

export function defaultSpirit(): SpiritInput {
  return {
    level: 159, quality: '神工', growth: '传说', stars: 4,
    huixin: 0, insight: 0, allocation: '力量', insightStat: '力量',
    fixedStat: '力量', huixinStat: '力量', divineTreasure: false, starGeneral: true,
    aptitude: { 力量: 2.65, 体质: 3.8, 心力: 2.63, 耐力: 2.74, 敏捷: 2.65 },
    treasureBase: { HP: 0, SP: 0, 攻击: 0, 防御: 0, 精神: 0, 闪避: 0 },
    treasureExtra: { HP: 0, SP: 0, 攻击: 0, 防御: 0, 精神: 0, 闪避: 0 },
    treasureFive: { 力量: 0, 体质: 0, 心力: 0, 耐力: 0, 敏捷: 0 },
  };
}

export function calculateSpirit(input: SpiritInput) {
  const invalid = (n: number | undefined) => typeof n !== 'number' || !Number.isFinite(n) || n < 0;
  if (!(spiritLevels as readonly number[]).includes(input.level)
    || !Object.hasOwn(qualityRates, input.quality) || !Object.hasOwn(qualityRates, input.growth)
    || invalid(input.stars) || !Number.isInteger(input.stars) || input.stars > 10)
    return { error: '请选择有效的等级、品质和强化星级。' } as const;
  if (invalid(input.huixin) || !Number.isInteger(input.huixin) || input.huixin! > 139
    || invalid(input.insight) || !Number.isInteger(input.insight) || input.insight! > 139)
    return { error: '慧心和领悟点数须为 0–139 的整数。' } as const;
  if ([input.allocation, input.insightStat, input.fixedStat, input.huixinStat].some(stat => !spiritDirections.includes(stat)))
    return { error: '请选择有效的加点方向。' } as const;
  if (spiritStats.some(stat => invalid(input.aptitude[stat])))
    return { error: '请填写五项资质，数值须大于或等于 0；灵宝对资质的影响请直接计入。' } as const;
  if ([...Object.values(input.treasureBase), ...Object.values(input.treasureExtra), ...Object.values(input.treasureFive)]
    .some(n => n !== undefined && invalid(n)))
    return { error: '灵宝属性须为大于或等于 0 的有效数字，留空按 0 计算。' } as const;

  const h = input.huixin!;
  const fixed = (h > 108 ? h * 3 : h > 72 ? h * 2 : h > 36 ? h : h * 0.5) * Number(input.divineTreasure);
  const assigned = h > 108 ? h * 2 : h > 36 ? h : h * 0.5;
  const base = input.level === 0 ? 0 : input.level + 9;
  const points = input.level * 5;
  const total = {} as Record<SpiritStat, number>;
  for (const stat of spiritStats) {
    total[stat] = base + (input.treasureFive[stat] ?? 0)
      + (input.allocation === stat ? points : 0)
      + (input.insightStat === stat ? input.insight! : 0)
      + (input.fixedStat === stat ? fixed : 0)
      + (input.huixinStat === stat ? assigned : 0);
  }
  const treasure = (stat: TreasureStat) => (input.treasureBase[stat] ?? 0) + (input.treasureExtra[stat] ?? 0);
  const aptitude = input.aptitude as Record<SpiritStat, number>;
  const panel = {
    HP: total.体质 * aptitude.体质 * 6 + treasure('HP'),
    SP: total.心力 * aptitude.心力 * 3 + treasure('SP'),
    攻击: total.力量 * aptitude.力量 + treasure('攻击'),
    防御: total.耐力 * aptitude.耐力 + treasure('防御'),
    灵巧: total.敏捷 * aptitude.敏捷 * 0.7,
    命中: total.力量 * aptitude.力量,
    精神: (total.心力 + (total.力量 + total.体质 + total.耐力) * 0.2) * aptitude.心力 + treasure('精神'),
    闪避: total.敏捷 * aptitude.敏捷 + treasure('闪避'),
  };
  const ratio = spiritRatio(input.quality, input.growth, input.stars)!;
  const factor = input.starGeneral ? 1.1 : 1;
  const rows = Object.entries(panel).map(([name, value]) => ({ name, panel: value, bonus: value * ratio * factor }));
  if (Object.values(total).some(n => !Number.isFinite(n)) || rows.some(row => !Number.isFinite(row.panel) || !Number.isFinite(row.bonus)))
    return { error: '计算结果超出数值范围，请减小输入。' } as const;
  return { error: null, total, rows, ratio, factor, base, points, fixed, assigned } as const;
}
