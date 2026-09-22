import { beforeEach, expect, test, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }))

import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import {
  BROKER_COMMANDS,
  DEFAULT_RUNTIME_LOG_LINES,
  MAX_RUNTIME_LOG_LINES,
  PROVISIONING_EVENT,
  getRuntimeLogs,
  getRuntimeStatus,
  listenProvisioningProgress,
  normalizeRuntimeLogLines,
  provisionRuntime,
  repairRuntime,
  startRuntime,
  stopRuntime,
  verifyRuntime,
} from './bridge'

beforeEach(() => vi.clearAllMocks())

test('renderer exposes only the enumerated bounded runtime commands', () => {
  expect(BROKER_COMMANDS).toEqual({
    runtimeStatus: 'runtime_status',
    runtimeProvision: 'runtime_provision',
    runtimeStart: 'runtime_start',
    runtimeStop: 'runtime_stop',
    runtimeVerify: 'runtime_verify',
    runtimeRepair: 'runtime_repair',
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
  await expect(verifyRuntime()).resolves.toBe(expected)
  await expect(repairRuntime()).resolves.toBe(expected)

  expect(invoke).toHaveBeenNthCalledWith(1, 'runtime_status')
  expect(invoke).toHaveBeenNthCalledWith(2, 'runtime_provision')
  expect(invoke).toHaveBeenNthCalledWith(3, 'runtime_start')
  expect(invoke).toHaveBeenNthCalledWith(4, 'runtime_stop')
  expect(invoke).toHaveBeenNthCalledWith(5, 'runtime_verify')
  expect(invoke).toHaveBeenNthCalledWith(6, 'runtime_repair')
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

test('provisioning progress is delivered unwrapped, from one named event, and stays unsubscribable', async () => {
  const unlisten = vi.fn()
  let handler: ((event: { payload: unknown }) => void) | undefined
  vi.mocked(listen).mockImplementation(async (_event, cb) => {
    handler = cb as (event: { payload: unknown }) => void
    return unlisten
  })

  const received: unknown[] = []
  const stop = await listenProvisioningProgress((payload) => received.push(payload))

  // One event name, and it is the one the broker emits. A renderer listening
  // on a name nothing publishes fails silently: provisioning appears frozen.
  expect(listen).toHaveBeenCalledTimes(1)
  expect(vi.mocked(listen).mock.calls[0]![0]).toBe(PROVISIONING_EVENT)
  expect(PROVISIONING_EVENT).toBe('provisioning-progress')

  // The listener receives the payload, not the Tauri envelope around it. A
  // caller handed the envelope would read `step` as undefined and render a
  // blank step for every message.
  const payload = { step: 'install-docker', detail: 'Installing Docker inside the distro' }
  handler?.({ payload })
  handler?.({ payload: { step: 'swap', detail: 'done' } })
  expect(received).toEqual([payload, { step: 'swap', detail: 'done' }])

  // The unlisten handle is passed straight through, so a component that
  // unmounts mid-provision can actually detach.
  expect(stop).toBe(unlisten)
})
