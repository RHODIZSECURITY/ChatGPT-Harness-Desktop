export type SessionState = 'active' | 'archived'

export interface SessionItem {
  id: string
  title: string
  state: SessionState
  createdAt: string
  updatedAt?: string
}
