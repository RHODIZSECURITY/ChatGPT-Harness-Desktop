import { type ReactNode, useCallback, useState } from 'react'
import { Button, InputTextArea, Text } from '@opal/components'
import { SvgShield, SvgStop } from '@opal/icons'
import { findRenderer } from '../conversation/findRenderer'
import { activeApprovalRequest, groupPackets, isStreamComplete } from '../conversation/grouping'
import type { ApprovalRequest, Packet } from '../conversation/protocol'
import type { ComponentState } from '../runtime/types'

/**
 * The four explicit composer states:
 * - `idle`: Runtime ready, no stream running, input enabled.
 * - `streaming`: Model/tool stream running, input disabled, Send transformed into Stop.
 * - `approval_required`: Stream paused on a security approval request; takeover active.
 * - `runtime_unavailable`: Harness Core not running or checking; input disabled with reason.
 */
export type ComposerState =
  | 'idle'
  | 'streaming'
  | 'approval_required'
  | 'runtime_unavailable'

export interface ConversationProps {
  core: ComponentState | 'checking'
  packets?: Packet[]
  isStreaming?: boolean
  onSend?: (text: string) => void
  onStop?: () => void
  onApprove?: (requestId: string, decision: 'allow_once' | 'reject') => Promise<void> | void
  slots?: {
    controlBelt?: ReactNode
    leftActions?: ReactNode
    rightActions?: ReactNode
  }
}

/**
 * The conversation surface.
 *
 * Structured the way onyx structures its own — a scrolling transcript above a
 * pinned composer — and over the same streaming packet protocol, narrowed to
 * the packets an agent doing work emits. See `src/conversation/protocol.ts`
 * for what was adopted and what was left behind.
 *
 * The transcript is packets rather than messages because that is what the
 * stream produces: text, reasoning, a shell command and its output, a file
 * read, a tool call. Flattening them to a message body would mean deciding,
 * at parse time, what is worth keeping.
 */
export default function Conversation({
  core,
  packets = [],
  isStreaming = false,
  onSend,
  onStop,
  onApprove,
  slots,
}: ConversationProps) {
  const [draft, setDraft] = useState('')
  const [isApproving, setIsApproving] = useState(false)
  const [resolvedApprovals, setResolvedApprovals] = useState<Set<string>>(new Set())

  const groups = groupPackets(packets)

  // Detect pending approval from packet stream
  const activeApproval: ApprovalRequest | null = activeApprovalRequest(packets)
  const pendingApproval =
    activeApproval && !resolvedApprovals.has(activeApproval.request_id)
      ? activeApproval
      : null

  // Derive explicit composer state
  const isCoreReady = core === 'ready'
  const isStreamActive = isStreaming && !isStreamComplete(packets)

  let composerState: ComposerState
  if (!isCoreReady) {
    composerState = 'runtime_unavailable'
  } else if (pendingApproval) {
    composerState = 'approval_required'
  } else if (isStreamActive) {
    composerState = 'streaming'
  } else {
    composerState = 'idle'
  }

  const reason =
    core === 'checking'
      ? 'Checking the runtime…'
      : 'The Harness Core is not running. Provision the runtime to start a session.'

  const handleSend = useCallback(() => {
    const trimmed = draft.trim()
    if (trimmed === '' || composerState !== 'idle') return
    onSend?.(trimmed)
    setDraft('')
  }, [draft, composerState, onSend])

  const handleDecision = useCallback(
    async (decision: 'allow_once' | 'reject') => {
      if (!pendingApproval || isApproving) return
      const reqId = pendingApproval.request_id

      // Optimistic takeover resolution: mark as resolved immediately
      setIsApproving(true)
      setResolvedApprovals((prev) => new Set(prev).add(reqId))

      try {
        await onApprove?.(reqId, decision)
      } catch (_err) {
        // Rollback on rejection or network failure
        setResolvedApprovals((prev) => {
          const next = new Set(prev)
          next.delete(reqId)
          return next
        })
      } finally {
        setIsApproving(false)
      }
    },
    [pendingApproval, isApproving, onApprove],
  )

  return (
    <section aria-label="Conversation" className="flex h-full flex-col">
      <div className="flex-1 overflow-auto px-10 py-8">
        {groups.length === 0 ? (
          <div className="mx-auto max-w-2xl pt-10 text-center">
            <Text font="heading-h3" color="text-04">
              Nothing here yet
            </Text>
            <div className="mt-2">
              <Text font="main-content-muted" color="text-03">
                Connect a Project and open a coding Session to begin.
              </Text>
            </div>
          </div>
        ) : (
          <ol className="mx-auto flex max-w-2xl flex-col gap-6">
            {groups.map((group) => {
              const Renderer = findRenderer(group)
              // A group of only control packets renders nothing, and an empty
              // list item would still occupy a row.
              if (Renderer === null) return null
              return (
                <li key={`${group.turn_index}-${group.tab_index}`}>
                  <Renderer group={group} />
                </li>
              )
            })}
          </ol>
        )}
      </div>

      <div className="border-t border-border-01 px-10 py-4">
        {slots?.controlBelt && (
          <div className="mx-auto mb-2 max-w-2xl">{slots.controlBelt}</div>
        )}

        {composerState === 'approval_required' && pendingApproval ? (
          /* Approval takeover in the composer.
             Renders plain text nodes for action, detail, and arguments —
             NEVER through CompactMarkdown or HTML parsing to prevent
             prompt/argument injection exploits in approval confirmation UI. */
          <div
            role="alertdialog"
            aria-label="Security Approval Required"
            aria-busy={isApproving}
            className="mx-auto max-w-2xl rounded-lg border border-border-02 bg-background-01 p-4 shadow-sm"
          >
            <div className="flex items-start gap-3">
              <div className="mt-0.5 text-status-text-warning-05">
                <SvgShield size={20} />
              </div>
              <div className="flex-1 min-w-0">
                <Text font="main-ui-body" color="text-04">
                  {`Approval required: ${pendingApproval.action}`}
                </Text>
                {pendingApproval.detail && (
                  <div className="mt-1">
                    <Text font="secondary-body" color="text-03">
                      {pendingApproval.detail}
                    </Text>
                  </div>
                )}
                {pendingApproval.arguments && Object.keys(pendingApproval.arguments).length > 0 && (
                  <div className="mt-2 rounded bg-background-02 p-2 font-mono text-xs text-text-03 overflow-x-auto">
                    {/* Rendered strictly as preformatted text node */}
                    <pre className="whitespace-pre-wrap break-all">
                      {JSON.stringify(pendingApproval.arguments, null, 2)}
                    </pre>
                  </div>
                )}
              </div>
            </div>

            <div className="mt-4 flex items-center justify-end gap-2">
              <Button
                size="sm"
                prominence="secondary"
                disabled={isApproving}
                onClick={() => handleDecision('reject')}
              >
                Reject
              </Button>
              <Button
                size="sm"
                prominence="primary"
                disabled={isApproving}
                onClick={() => handleDecision('allow_once')}
              >
                Allow once
              </Button>
            </div>
          </div>
        ) : (
          <div className="mx-auto flex max-w-2xl items-end gap-2">
            {slots?.leftActions && (
              <div className="flex shrink-0 items-center gap-1">{slots.leftActions}</div>
            )}

            {/* Opal's inputs take no className — styling is the library's, not
                the caller's — and express unavailability as a variant rather
                than the DOM `disabled` attribute, so the field keeps its chrome
                and stays announced. Both are its API; the wrapper carries the
                layout instead. */}
            <div className="flex-1">
              <InputTextArea
                autoResize
                rows={1}
                maxRows={8}
                variant={composerState === 'idle' ? 'primary' : 'disabled'}
                placeholder="Ask the Harness…"
                aria-label="Message"
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' && !event.shiftKey) {
                    event.preventDefault()
                    handleSend()
                  }
                }}
              />
            </div>

            {slots?.rightActions && (
              <div className="flex shrink-0 items-center gap-1">{slots.rightActions}</div>
            )}

            {composerState === 'streaming' ? (
              <Button
                icon={SvgStop}
                prominence="secondary"
                aria-label="Stop generating"
                tooltip="Stop generating"
                tooltipSide="top"
                onClick={onStop}
              >
                Stop
              </Button>
            ) : (
              <Button
                disabled={composerState !== 'idle' || draft.trim() === ''}
                onClick={handleSend}
              >
                Send
              </Button>
            )}
          </div>
        )}

        {composerState === 'runtime_unavailable' && (
          <div className="mx-auto mt-2 max-w-2xl">
            <Text font="secondary-body" color="text-02">
              {reason}
            </Text>
          </div>
        )}
      </div>
    </section>
  )
}
