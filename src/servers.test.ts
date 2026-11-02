import { test } from 'node:test'
import assert from 'node:assert/strict'
import { serverOptions } from './servers.ts'
import type { Area } from './filter.ts'

test('merged server aliases are visible and searchable, default only fills an empty list', () => {
  const areas: Area[] = [
    {
      name: '电信一区',
      server: [
        { serverId: '104', name: '侠客岛' },
        { serverId: '104', name: '七星台' },
      ],
    },
  ]
  const options = serverOptions(areas, [])
  assert.equal(options.length, 1)
  assert.equal(options[0].label, '电信一区 · 侠客岛 / 七星台')
  assert.equal(serverOptions([], [])[0].value, '104')
  assert.deepEqual(
    serverOptions(
      [{ name: '新区', server: [{ serverId: '8501', name: '龙腾九霄' }] }],
      [],
    ).map((s) => s.value),
    ['8501'],
  )
})
