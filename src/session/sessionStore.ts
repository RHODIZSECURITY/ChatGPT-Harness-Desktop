import type { SessionItem } from './types'

export const SESSIONS_STORAGE_KEY = 'rhodiz:sessions:list'
export const ACTIVE_SESSION_STORAGE_KEY = 'rhodiz:sessions:active_id'

export const MAX_SESSIONS = 100
export const MAX_TITLE_LENGTH = 200
export const MAX_RAW_PAYLOAD_BYTES = 256 * 1024

export const DEFAULT_SESSION: SessionItem = {
  id: 'session-default',
  title: 'Current Session',
  state: 'active',
  createdAt: '2026-09-26T10:00:00Z',
}

function isValidId(val: unknown): val is string {
  return typeof val === 'string' && val.length > 0 && val.length <= 128 && /^[a-zA-Z0-9_-]+$/.test(val)
}

function normalizeTimestamp(val: unknown): string | null {
  if (typeof val === 'string' && val.length <= 50) {
    const parsed = Date.parse(val)
    if (!Number.isNaN(parsed)) return val
  } else if (typeof val === 'number' && Number.isFinite(val) && val > 0) {
    return new Date(val).toISOString()
  }
  return null
}

export function sanitizeTitle(raw: unknown): string | null {
  if (typeof raw !== 'string') return null
  // Filter control characters (< 32) and collapse whitespace
  const chars = Array.from(raw).map((c) => (c.charCodeAt(0) < 32 ? ' ' : c))
  const normalized = chars.join('').replace(/\s+/g, ' ').trim()
  if (normalized.length === 0) return null
  return normalized.slice(0, MAX_TITLE_LENGTH)
}

function validateAndSanitizeSession(item: unknown): SessionItem | null {
  if (typeof item !== 'object' || item === null) return null
  const candidate = item as Record<string, unknown>

  if (!isValidId(candidate.id)) return null

  const cleanTitle = sanitizeTitle(candidate.title)
  if (!cleanTitle) return null

  // Backward compatibility: default missing or unrecognized state to 'active'
  const state: 'active' | 'archived' = candidate.state === 'archived' ? 'archived' : 'active'

  // Backward compatibility: normalize string or numeric epoch timestamps
  const createdAt = normalizeTimestamp(candidate.createdAt) || new Date().toISOString()

  const sanitized: SessionItem = {
    id: candidate.id,
    title: cleanTitle,
    state,
    createdAt,
  }

  const updatedAt = normalizeTimestamp(candidate.updatedAt)
  if (updatedAt) {
    sanitized.updatedAt = updatedAt
  }

  return sanitized
}

export function loadSessions(): SessionItem[] {
  try {
    const raw = localStorage.getItem(SESSIONS_STORAGE_KEY)
    if (raw) {
      if (raw.length > MAX_RAW_PAYLOAD_BYTES) {
        return [DEFAULT_SESSION]
      }
      const parsed = JSON.parse(raw)
      if (Array.isArray(parsed) && parsed.length > 0) {
        const validated: SessionItem[] = []
        const seenIds = new Set<string>()
        for (const item of parsed) {
          const session = validateAndSanitizeSession(item)
          if (session && !seenIds.has(session.id)) {
            seenIds.add(session.id)
            validated.push(session)
            if (validated.length >= MAX_SESSIONS) break
          }
        }
        if (validated.length > 0) {
          return validated
        }
      }
    }
  } catch (_err) {
    // Fall back fail-closed to default session if corrupted or unreadable
  }
  return [DEFAULT_SESSION]
}

export function saveSessions(sessions: SessionItem[]): void {
  try {
    const seenIds = new Set<string>()
    const sanitized: SessionItem[] = []
    for (const s of sessions) {
      const valid = validateAndSanitizeSession(s)
      if (valid && !seenIds.has(valid.id)) {
        seenIds.add(valid.id)
        sanitized.push(valid)
        if (sanitized.length >= MAX_SESSIONS) break
      }
    }
    localStorage.setItem(SESSIONS_STORAGE_KEY, JSON.stringify(sanitized))
  } catch (err) {
    if (typeof console !== 'undefined' && console.warn) {
      console.warn('Failed to persist sessions to localStorage:', err)
    }
  }
}

export function loadActiveSessionId(validSessionIds?: string[]): string {
  try {
    const active = localStorage.getItem(ACTIVE_SESSION_STORAGE_KEY)
    if (active && isValidId(active)) {
      if (!validSessionIds || validSessionIds.includes(active)) {
        return active
      }
    }
  } catch (_err) {
    // Non-blocking fallback
  }
  return validSessionIds && validSessionIds.length > 0 ? validSessionIds[0] : DEFAULT_SESSION.id
}

export function saveActiveSessionId(id: string, validSessionIds?: string[]): void {
  try {
    if (!isValidId(id)) return
    if (validSessionIds && !validSessionIds.includes(id)) return
    localStorage.setItem(ACTIVE_SESSION_STORAGE_KEY, id)
  } catch (err) {
    if (typeof console !== 'undefined' && console.warn) {
      console.warn('Failed to persist active session ID to localStorage:', err)
    }
  }
}

export function createNewSession(title?: string): SessionItem {
  const now = new Date().toISOString()
  let uuid: string
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    uuid = crypto.randomUUID()
  } else {
    uuid = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
      const r = (Math.random() * 16) | 0
      const v = c === 'x' ? r : (r & 0x3) | 0x8
      return v.toString(16)
    })
  }
  const id = `session-${uuid}`
  const cleanTitle = sanitizeTitle(title) || 'New Session'

  return {
    id,
    title: cleanTitle,
    state: 'active',
    createdAt: now,
    updatedAt: now,
  }
}
