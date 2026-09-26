import type { ComponentState } from '../runtime/types'

/**
 * The status colour for live runtime states, shared by the window footer and
 * the composer's context strip.
 *
 * `checking` is deliberately not a state the broker can return — it is the
 * gap before the first answer, and must not look like a verdict.
 *
 * Shared rather than copied: two tone maps drift, and drift here means the
 * same state reading as success in one strip and as noise in the other.
 */
export const STATE_TONE: Record<ComponentState | 'checking', string> = {
  ready: 'text-status-text-success-05',
  stopped: 'text-status-text-warning-05',
  missing: 'text-status-text-error-05',
  unavailable: 'text-text-02',
  checking: 'text-text-02',
}
