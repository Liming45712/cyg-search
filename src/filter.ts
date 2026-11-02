export interface Attr { raw: string; num: number | null }
export interface Role {
  code: string; name: string; title: string; job: string; price: number | null; server_id: string;
  status: number; listed_at: string; offline_at: string; checked_at: number;
  detail: { attrs: Record<string, Attr>; equipment: Record<string, number> } | null;
  detail_at: number | null; detail_error: string | null;
  needs_refresh?: boolean;
}
export interface Area { name: string; server: { serverId: string; name: string }[] }
export interface Filters {
  keyword: string; server: string; jobs: string[]; status: string;
  price: [number | null, number | null]; minimums: Record<string, number | null>;
  equipment: string[]; matchAll: boolean; sort: string; desc: boolean;
}
export const primary = ['等级', '装备总评分', '铭石总等级', '星石总等级', '物理抗性', '法术抗性', '异常抗性']
export const equipTags = ['至尊武器', '神佑武器', '至尊项链', '神佑项链', '至尊戒指', '神佑戒指', '至尊防具', '神佑防具']
export function defaults(server = '104'): Filters {
  return { keyword: '', server, jobs: [], status: '', price: [null, null], minimums: {}, equipment: [], matchAll: true, sort: 'price', desc: false }
}
export function numeric(role: Role, key: string): number | null {
  return key === 'price' ? role.price : role.detail?.attrs[key]?.num ?? null
}
export function rangeError(filters: Filters): string | null {
  if (filters.price.some(n => n !== null && n < 0)) return '价格不能小于零'
  const [lo, hi] = filters.price
  if (filters.price.some(n => n !== null && !Number.isFinite(n))) return '价格请输入有效数字'
  if (lo !== null && hi !== null && lo > hi) return '价格的下限不能大于上限'
  for (const [key, minimum] of Object.entries(filters.minimums))
    if (minimum !== null && !Number.isFinite(minimum)) return `${key}请输入有效数字`
  return null
}
export function applyFilters(roles: Role[], f: Filters): Role[] {
  if (rangeError(f)) return []
  const kw = f.keyword.trim().toLocaleLowerCase()
  const inRange = (v: number | null, [lo, hi]: [number | null, number | null]) =>
    (lo === null && hi === null) || (v !== null && (lo === null || v >= lo) && (hi === null || v <= hi))
  return roles.filter(r => {
    if (f.server && r.server_id !== f.server) return false
    if (kw && !r.name.toLocaleLowerCase().includes(kw)) return false
    if (f.jobs.length && !f.jobs.includes(r.job)) return false
    if (f.status && r.status !== Number(f.status)) return false
    if (!inRange(r.price, f.price)) return false
    if (!Object.entries(f.minimums).every(([key, minimum]) => {
      if (minimum === null) return true
      const value = numeric(r, key)
      return value !== null && value >= minimum
    })) return false
    const hits = f.equipment.map(tag => (r.detail?.equipment[tag] ?? 0) > 0)
    return !hits.length || (f.matchAll ? hits.every(Boolean) : hits.some(Boolean))
  }).sort((a, b) => {
    let av: number | string | null, bv: number | string | null
    if (f.sort === 'listed_at' || f.sort === 'offline_at') {
      av = a[f.sort] || null; bv = b[f.sort] || null
    } else { av = numeric(a, f.sort); bv = numeric(b, f.sort) }
    // Missing attributes always sort last, including descending order.
    if (av === null) return bv === null ? a.code.localeCompare(b.code) : 1
    if (bv === null) return -1
    const diff = typeof av === 'number' && typeof bv === 'number' ? av - bv : String(av).localeCompare(String(bv))
    return (f.desc ? -diff : diff) || a.code.localeCompare(b.code)
  })
}
