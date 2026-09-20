import { invoke } from '@tauri-apps/api/core'
import type { RuntimeStatus } from './types'

export const BROKER_COMMANDS = Object.freeze({
  runtimeStatus: 'runtime_status',
} as const)

export async function getRuntimeStatus(): Promise<RuntimeStatus> {
  return invoke<RuntimeStatus>(BROKER_COMMANDS.runtimeStatus)
}