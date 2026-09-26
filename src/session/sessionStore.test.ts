import { beforeEach, describe, expect, test } from 'vitest'
import {
  ACTIVE_SESSION_STORAGE_KEY,
  DEFAULT_SESSION,
  MAX_SESSIONS,
  MAX_TITLE_LENGTH,
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

  test('loadActiveSessionId returns default ID when empty or when stored ID not in valid list', () => {
    expect(loadActiveSessionId()).toBe(DEFAULT_SESSION.id)

    localStorage.setItem(ACTIVE_SESSION_STORAGE_KEY, 'non-existent-id')
    expect(loadActiveSessionId(['s1', 's2'])).toBe('s1')
  })

  test('saveActiveSessionId only stores IDs in validSessionIds when provided', () => {
    saveActiveSessionId('s-custom-123')
    expect(loadActiveSessionId()).toBe('s-custom-123')

    saveActiveSessionId('attacker-id', ['allowed-1', 'allowed-2'])
    // Stored value remains previous since attacker-id is rejected
    expect(loadActiveSessionId()).toBe('s-custom-123')
  })

  test('createNewSession generates unique active session with valid UUID format', () => {
    const s1 = createNewSession('Feature X')
    const s2 = createNewSession()

    expect(s1.title).toBe('Feature X')
    expect(s1.state).toBe('active')
    expect(s1.id).toMatch(/^session-[0-9a-f-]{36}$/)

    expect(s2.title).toBe('New Session')
    expect(s2.state).toBe('active')
    expect(s1.id).not.toBe(s2.id)
  })

  test('handles corrupted localStorage JSON gracefully', () => {
    localStorage.setItem(SESSIONS_STORAGE_KEY, 'not-valid-json{')
    expect(loadSessions()).toEqual([DEFAULT_SESSION])
  })

  test('adversarial validation: rejects malformed JSON items and limits capacity', () => {
    // Malicious or corrupted payloads
    const maliciousPayload = [
      { id: 123, title: {}, state: 'pwned' },
      { id: 'valid-1', title: 'Safe Session', state: 'active', createdAt: '2026-09-26T10:00:00Z' },
      { id: 'x', title: { __brand: 'RichStr', raw: '[x](https://evil)' }, state: 'active' },
      { id: 'too-long-id'.repeat(30), title: 'Valid', state: 'active', createdAt: '2026-09-26T10:00:00Z' },
      null,
      'a string',
    ]

    localStorage.setItem(SESSIONS_STORAGE_KEY, JSON.stringify(maliciousPayload))
    const loaded = loadSessions()
    expect(loaded).toHaveLength(1)
    expect(loaded[0].id).toBe('valid-1')
    expect(loaded[0].title).toBe('Safe Session')

    // Bounded capacity: cannot exceed MAX_SESSIONS
    const oversized: SessionItem[] = Array.from({ length: 150 }, (_, i) => ({
      id: `s-${i}`,
      title: `Session ${i}`,
      state: 'active' as const,
      createdAt: '2026-09-26T10:00:00Z',
    }))
    saveSessions(oversized)
    const capped = loadSessions()
    expect(capped.length).toBe(MAX_SESSIONS)
  })

  test('backward compatibility: truncates oversized titles and migrates numeric timestamps', () => {
    const legacyItem = {
      id: 'legacy-1',
      title: 'A'.repeat(250), // exceeds 200 chars
      createdAt: 1727344800000, // numeric epoch
      updatedAt: '2026-09-26T12:00:00Z',
    }

    localStorage.setItem(SESSIONS_STORAGE_KEY, JSON.stringify([legacyItem]))
    const loaded = loadSessions()
    expect(loaded).toHaveLength(1)
    expect(loaded[0].id).toBe('legacy-1')
    expect(loaded[0].title.length).toBe(MAX_TITLE_LENGTH)
    expect(loaded[0].state).toBe('active') // default assigned
    expect(loaded[0].createdAt).toBe(new Date(1727344800000).toISOString())
  })
})

  test('deduplicates duplicate IDs in localStorage payload', () => {
    const payload = [
      { id: 'dup-1', title: 'First Instance', state: 'active', createdAt: '2026-09-26T10:00:00Z' },
      { id: 'dup-1', title: 'Second Instance', state: 'active', createdAt: '2026-09-26T11:00:00Z' },
      { id: 'unique-2', title: 'Other Session', state: 'active', createdAt: '2026-09-26T12:00:00Z' },
    ]
    localStorage.setItem(SESSIONS_STORAGE_KEY, JSON.stringify(payload))
    const loaded = loadSessions()
    expect(loaded).toHaveLength(2)
    expect(loaded[0].title).toBe('First Instance')
    expect(loaded[1].title).toBe('Other Session')
  })

  test('sanitizes control characters and collapses multi-whitespace in title', () => {
    const item = {
      id: 'ctrl-1',
      title: 'Hello\n\tWorld\r   Test\x00!',
      state: 'active',
      createdAt: '2026-09-26T10:00:00Z',
    }
    localStorage.setItem(SESSIONS_STORAGE_KEY, JSON.stringify([item]))
    const loaded = loadSessions()
    expect(loaded[0].title).toBe('Hello World Test !')
  })
