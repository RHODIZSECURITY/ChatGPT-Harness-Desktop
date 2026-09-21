export type ComponentState = 'ready' | 'stopped' | 'missing' | 'unavailable'

export interface RuntimeComponent {
  state: ComponentState
  detail?: string
}

export interface RuntimeStatus {
  platform: 'windows' | 'unsupported'
  wsl2: RuntimeComponent
  docker: RuntimeComponent
  core: RuntimeComponent
  route: RuntimeComponent
  memory: RuntimeComponent
  providers: RuntimeComponent
}

export type RuntimeOperation = 'provision' | 'start' | 'stop' | 'verify' | 'repair' | 'logs'
export type OperationState =
  | 'succeeded'
  | 'failed'
  | 'blocked'
  | 'timed_out'
  | 'unsupported'

export interface RuntimeOperationResult {
  operation: RuntimeOperation
  state: OperationState
  detail?: string
}

export interface RuntimeLogsResult {
  state: OperationState
  lines: string[]
  truncated: boolean
  detail?: string
}

export type UnitState =
  | 'active'
  | 'inactive'
  | 'distro_unreachable'
  | 'wsl_missing'
  | 'unknown'

export interface RuntimeVerifyResult {
  operation: 'verify'
  unit: UnitState
  /** False whenever the broker could not determine the unit state. */
  healthy: boolean
  detail: string
  /** Components verification does not probe yet, so the renderer cannot
   *  mistake "not probed" for "verified healthy". */
  unprobed: string[]
}
