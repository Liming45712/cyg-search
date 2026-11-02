import type { Area, Role } from './filter'

export function serverOptions(areas: Area[], roles: Role[]) {
  const servers = new Map<string, { area: string; names: Set<string> }>()
  for (const area of areas)
    for (const server of area.server) {
      const item = servers.get(server.serverId) ?? {
        area: area.name,
        names: new Set<string>(),
      }
      item.names.add(server.name)
      servers.set(server.serverId, item)
    }
  for (const role of roles)
    if (!servers.has(role.server_id)) {
      servers.set(role.server_id, {
        area: '已缓存区服',
        names: new Set([role.server_id]),
      })
    }
  if (!servers.size)
    servers.set('104', { area: '电信一区', names: new Set(['侠客岛']) })
  return [...servers].map(([value, server]) => {
    const name = [...server.names].join(' / ')
    return { value, area: server.area, name, label: `${server.area} · ${name}` }
  })
}
