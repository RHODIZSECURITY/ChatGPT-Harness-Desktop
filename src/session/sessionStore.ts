import type { SessionItem } from './types'

export const SESSIONS_STORAGE_KEY = 'rhodiz:sessions:list'
export const ACTIVE_SESSION_STORAGE_KEY = 'rhodiz:sessions:active_id'

export const DEFAULT_SESSION: SessionItem = {
  id: 'session-default',
  title: 'Current Session',
  state: 'active',
  createdAt: '2026-09-26T10:00:00Z',
}

export function loadSessions(): SessionItem[] {
  try {
    const raw = localStorage.getItem(SESSIONS_STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw)
      if (Array.isArray(parsed) && parsed.length > 0) {
        return parsed
      }
    }
  } catch (_err) {
    // Fall back to default session if corrupted or unreadable
  }
  return [DEFAULT_SESSION]
}

export function saveSessions(sessions: SessionItem[]): void {
  try {
    localStorage.setItem(SESSIONS_STORAGE_KEY, JSON.stringify(sessions))
  } catch (_err) {
    // Non-blocking storage write
  }
}

export function loadActiveSessionId(): string {
  try {
    const active = localStorage.getItem(ACTIVE_SESSION_STORAGE_KEY)
    if (active) return active
  } catch (_err) {
    // Non-blocking fallback
  }
  return DEFAULT_SESSION.id
}

export function saveActiveSessionId(id: string): void {
  try {
    localStorage.setItem(ACTIVE_SESSION_STORAGE_KEY, id)
  } catch (_err) {
    // Non-blocking fallback
  }
}

export function createNewSession(title?: string): SessionItem {
  const now = new Date().toISOString()
  const randomSuffix = Math.random().toString(36).substring(2, 7)
  const id = `session-${Date.now()}-${randomSuffix}`
  return {
    id,
    title: title || 'New Session',
    state: 'active',
    createdAt: now,
    updatedAt: now,
  }
}
