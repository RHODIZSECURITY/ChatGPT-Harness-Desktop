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

export type RuntimeOperation = 'provision' | 'start' | 'stop' | 'logs'
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
