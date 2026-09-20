import { beforeEach, expect, test, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

import { invoke } from '@tauri-apps/api/core'
import {
  BROKER_COMMANDS,
  DEFAULT_RUNTIME_LOG_LINES,
  MAX_RUNTIME_LOG_LINES,
  getRuntimeLogs,
  getRuntimeStatus,
  normalizeRuntimeLogLines,
  provisionRuntime,
  startRuntime,
  stopRuntime,
} from './bridge'

beforeEach(() => vi.clearAllMocks())

test('renderer exposes only the enumerated bounded runtime commands', () => {
  expect(BROKER_COMMANDS).toEqual({
    runtimeStatus: 'runtime_status',
    runtimeProvision: 'runtime_provision',
    runtimeStart: 'runtime_start',
    runtimeStop: 'runtime_stop',
    runtimeLogs: 'runtime_logs',
  })
})

test('status and lifecycle mutations carry no renderer-controlled arguments', async () => {
  const expected = { state: 'succeeded' }
  vi.mocked(invoke).mockResolvedValue(expected)

  await expect(getRuntimeStatus()).resolves.toBe(expected)
  await expect(provisionRuntime()).resolves.toBe(expected)
  await expect(startRuntime()).resolves.toBe(expected)
  await expect(stopRuntime()).resolves.toBe(expected)

  expect(invoke).toHaveBeenNthCalledWith(1, 'runtime_status')
  expect(invoke).toHaveBeenNthCalledWith(2, 'runtime_provision')
  expect(invoke).toHaveBeenNthCalledWith(3, 'runtime_start')
  expect(invoke).toHaveBeenNthCalledWith(4, 'runtime_stop')
})

test('log requests clamp non-finite, fractional, low and high values', async () => {
  vi.mocked(invoke).mockResolvedValue({ state: 'succeeded', lines: [], truncated: false })

  expect(normalizeRuntimeLogLines(Number.NaN)).toBe(DEFAULT_RUNTIME_LOG_LINES)
  expect(normalizeRuntimeLogLines(Number.POSITIVE_INFINITY)).toBe(DEFAULT_RUNTIME_LOG_LINES)
  expect(normalizeRuntimeLogLines(0)).toBe(1)
  expect(normalizeRuntimeLogLines(12.9)).toBe(12)
  expect(normalizeRuntimeLogLines(9_999)).toBe(MAX_RUNTIME_LOG_LINES)

  await getRuntimeLogs(Number.NaN)
  await getRuntimeLogs(0)
  await getRuntimeLogs(12.9)
  await getRuntimeLogs(9_999)

  expect(invoke).toHaveBeenNthCalledWith(1, 'runtime_logs', { lines: 200 })
  expect(invoke).toHaveBeenNthCalledWith(2, 'runtime_logs', { lines: 1 })
  expect(invoke).toHaveBeenNthCalledWith(3, 'runtime_logs', { lines: 12 })
  expect(invoke).toHaveBeenNthCalledWith(4, 'runtime_logs', { lines: 500 })
})
