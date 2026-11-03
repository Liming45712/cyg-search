import type { Attr, Role } from './filter';
import { spiritRatio } from './spirit';

export const spiritPanelFields = [
  { name: 'HP', source: '生命值' }, { name: 'SP', source: '技能值' },
  { name: '攻击', source: '攻击力' }, { name: '防御', source: '防御力' },
  { name: '精神', source: '精神力' }, { name: '灵巧', source: '灵巧度' },
  { name: '命中', source: '命中率' }, { name: '闪避', source: '闪避率' },
] as const;
export const spiritAptitudes = ['力量资质', '体力资质', '心力资质', '耐力资质', '敏捷资质'];

export function marketSpirit(item: Role, starGeneral: boolean): Role {
  // Keep cached panel and its quality/strengthening from the same detail snapshot.
  const attrs: Record<string, Attr> = { ...(item.detail?.attrs ?? item.list_attrs ?? {}) };
  const ratio = spiritRatio(attrs.资质?.raw ?? '', attrs.成长?.raw ?? '', attrs.强化?.num);
  attrs.加成比例 = { raw: ratio === null ? '缺少品质、成长或强化' : `${ratio * 100}%`, num: ratio === null ? null : ratio * 100 };
  for (const field of spiritPanelFields) {
    const panel = item.detail?.attrs[field.source]?.num;
    const value = ratio !== null && typeof panel === 'number' && Number.isFinite(panel) && panel >= 0
      ? panel * ratio * (starGeneral ? 1.1 : 1) : null;
    const num = value !== null && Number.isFinite(value) ? value : null;
    attrs[`加成${field.name}`] = { raw: num === null ? '待获取完整详情' : String(num), num };
  }
  return { ...item, detail: { attrs, equipment: {} } };
}
