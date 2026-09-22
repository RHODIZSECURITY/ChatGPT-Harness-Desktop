import { useState } from 'react'
import { Button, InputTextArea, Text } from '@opal/components'
import { findRenderer } from '../conversation/findRenderer'
import { groupPackets } from '../conversation/grouping'
import type { Packet } from '../conversation/protocol'
import type { ComponentState } from '../runtime/types'

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
  // Nothing produces packets yet: the send path needs a provisioned runtime to
  // send to. They arrive as a prop rather than as state owned here so the
  // transcript renders whatever it is given — which is how it can be exercised
  // before a stream exists to give it anything.
  packets = [],
}: {
  core: ComponentState | 'checking'
  packets?: Packet[]
}) {
  const [draft, setDraft] = useState('')

  const groups = groupPackets(packets)

  // The composer is gated on the runtime, not on a feature flag: with the
  // Harness Core down there is nothing to send to, and an input that accepts
  // text it will silently drop is worse than one that says why it cannot.
  const ready = core === 'ready'
  const reason =
    core === 'checking'
      ? 'Checking the runtime…'
      : 'The Harness Core is not running. Provision the runtime to start a session.'

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
        <div className="mx-auto flex max-w-2xl items-end gap-2">
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
              variant={ready ? 'primary' : 'disabled'}
              // The reason belongs under the field, once. Repeating it as the
              // placeholder reads as a rendering fault, and a placeholder
              // disappears the moment anyone types — which is exactly when a
              // disabled field has the least to say.
              placeholder="Ask the Harness…"
              aria-label="Message"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
            />
          </div>
          <Button disabled={!ready || draft.trim() === ''}>Send</Button>
        </div>
        {!ready && (
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
