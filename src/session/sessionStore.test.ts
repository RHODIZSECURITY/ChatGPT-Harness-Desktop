import { beforeEach, describe, expect, test } from 'vitest'
import {
  DEFAULT_SESSION,
  SESSIONS_STORAGE_KEY,
  createNewSession,
  loadActiveSessionId,
  loadSessions,
  saveActiveSessionId,
  saveSessions,
} from './sessionStore'
import type { SessionItem } from './types'

describe('sessionStore', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  test('loadSessions returns default session when storage is empty', () => {
    const sessions = loadSessions()
    expect(sessions).toEqual([DEFAULT_SESSION])
  })

  test('saveSessions and loadSessions round-trip data correctly', () => {
    const testSessions: SessionItem[] = [
      { id: 's1', title: 'Task 1', state: 'active', createdAt: '2026-09-26T10:00:00Z' },
      { id: 's2', title: 'Task 2', state: 'archived', createdAt: '2026-09-25T10:00:00Z' },
    ]

    saveSessions(testSessions)
    const loaded = loadSessions()
    expect(loaded).toEqual(testSessions)
  })

  test('loadActiveSessionId returns default ID when empty', () => {
    expect(loadActiveSessionId()).toBe(DEFAULT_SESSION.id)
  })

  test('saveActiveSessionId stores and retrieves active ID', () => {
    saveActiveSessionId('s-custom-123')
    expect(loadActiveSessionId()).toBe('s-custom-123')
  })

  test('createNewSession generates unique active session with timestamp', () => {
    const s1 = createNewSession('Feature X')
    const s2 = createNewSession()

    expect(s1.title).toBe('Feature X')
    expect(s1.state).toBe('active')
    expect(s1.id).toMatch(/^session-\d+-[a-z0-9]+$/)

    expect(s2.title).toBe('New Session')
    expect(s2.state).toBe('active')
    expect(s1.id).not.toBe(s2.id)
  })

  test('handles corrupted localStorage JSON gracefully', () => {
    localStorage.setItem(SESSIONS_STORAGE_KEY, 'not-valid-json{')
    expect(loadSessions()).toEqual([DEFAULT_SESSION])
  })
})
