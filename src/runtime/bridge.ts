import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  ProvisioningProgressPayload,
  RuntimeLogsResult,
  RuntimeOperationResult,
  RuntimeStatus,
  RuntimeVerifyResult,
} from './types'

export const DEFAULT_RUNTIME_LOG_LINES = 200
export const MAX_RUNTIME_LOG_LINES = 500

export const BROKER_COMMANDS = Object.freeze({
  runtimeStatus: 'runtime_status',
  runtimeProvision: 'runtime_provision',
  runtimeStart: 'runtime_start',
  runtimeStop: 'runtime_stop',
  runtimeVerify: 'runtime_verify',
  runtimeRepair: 'runtime_repair',
  runtimeLogs: 'runtime_logs',
} as const)

export const PROVISIONING_EVENT = 'provisioning-progress'

export function normalizeRuntimeLogLines(lines = DEFAULT_RUNTIME_LOG_LINES): number {
  if (!Number.isFinite(lines)) return DEFAULT_RUNTIME_LOG_LINES
  return Math.min(MAX_RUNTIME_LOG_LINES, Math.max(1, Math.trunc(lines)))
}

export async function getRuntimeStatus(): Promise<RuntimeStatus> {
  return invoke<RuntimeStatus>(BROKER_COMMANDS.runtimeStatus)
}

export async function provisionRuntime(): Promise<RuntimeOperationResult> {
  return invoke<RuntimeOperationResult>(BROKER_COMMANDS.runtimeProvision)
}

export async function startRuntime(): Promise<RuntimeOperationResult> {
  return invoke<RuntimeOperationResult>(BROKER_COMMANDS.runtimeStart)
}

export async function stopRuntime(): Promise<RuntimeOperationResult> {
  return invoke<RuntimeOperationResult>(BROKER_COMMANDS.runtimeStop)
}

export async function verifyRuntime(): Promise<RuntimeVerifyResult> {
  return invoke<RuntimeVerifyResult>(BROKER_COMMANDS.runtimeVerify)
}

export async function repairRuntime(): Promise<RuntimeOperationResult> {
  return invoke<RuntimeOperationResult>(BROKER_COMMANDS.runtimeRepair)
}

export async function getRuntimeLogs(
  lines = DEFAULT_RUNTIME_LOG_LINES,
): Promise<RuntimeLogsResult> {
  return invoke<RuntimeLogsResult>(BROKER_COMMANDS.runtimeLogs, {
    lines: normalizeRuntimeLogLines(lines),
  })
}

export type ProvisioningProgressListener = (payload: ProvisioningProgressPayload) => void

export async function listenProvisioningProgress(
  listener: ProvisioningProgressListener,
): Promise<import('@tauri-apps/api/event').UnlistenFn> {
  return listen<ProvisioningProgressPayload>(PROVISIONING_EVENT, (event) => {
    listener(event.payload)
  })
}
