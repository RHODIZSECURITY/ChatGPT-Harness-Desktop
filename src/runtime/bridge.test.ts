import { beforeEach, expect, test, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

import { invoke } from '@tauri-apps/api/core'
import { BROKER_COMMANDS, getRuntimeStatus } from './bridge'

beforeEach(() => vi.clearAllMocks())

test('renderer invokes only the enumerated runtime status command', async () => {
  const expected = { platform: 'unsupported' }
  vi.mocked(invoke).mockResolvedValue(expected)
  await expect(getRuntimeStatus()).resolves.toBe(expected)
  expect(BROKER_COMMANDS).toEqual({ runtimeStatus: 'runtime_status' })
  expect(invoke).toHaveBeenCalledTimes(1)
  expect(invoke).toHaveBeenCalledWith('runtime_status')
})
