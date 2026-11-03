import type { Role } from './filter';

export type Stat = [name: string, value: number, unit?: string];
export interface GearDetail {
  kind: string;
  level: number | null;
  score: number | null;
  stats: Stat[];
  effects: string[];
  set: string[];
  raw: string[];
}
export interface Item {
  code: string; name: string; type: string; quality: string;
  level: number | null; score: number | null; price: number | null;
  stats: Stat[]; effects: string[]; set?: string[]; raw: string[];
  status: number; saleTime: string; offlineTime: string;
  ready: boolean; error: string | null; needsRefresh: boolean;
}
export function equipmentAttribute(name: string): string {
  return ({ 基本攻击力: '攻击力', 基本精神力: '精神力', 基本防御力: '防御力' } as Record<string, string>)[name] ?? name;
}
export function marketEquipment(row: Role): Item {
  const gear = row.detail?.gear;
  return {
    code: row.code, name: row.name, type: gear?.kind ?? '待获取类型',
    quality: row.list_attrs?.装备品级?.raw ?? '未知',
    level: gear?.level ?? row.list_attrs?.使用等级?.num ?? null,
    score: gear?.score ?? row.list_attrs?.装备评分?.num ?? null,
    price: row.price,
    stats: gear?.stats.map(([name, value, unit])=>[equipmentAttribute(name),value,unit]) ?? [],
    effects: gear?.effects ?? [], set: gear?.set.length ? gear.set : undefined, raw: gear?.raw ?? [],
    status: row.status, saleTime: row.listed_at, offlineTime: row.offline_at,
    ready: !!gear, error: row.detail_error, needsRefresh: row.needs_refresh ?? false,
  };
}
export function equipmentMinimum(item: Item, name: string, minimum?: number): boolean {
  return !name || minimum === undefined || item.stats.some(s=>s[0]===name && s[1]>=minimum);
}
