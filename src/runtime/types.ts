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