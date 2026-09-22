import { useState } from 'react'
import { Button, InputTextArea, Text } from '@opal/components'
import type { ComponentState } from '../runtime/types'

export interface ConversationMessage {
  id: string
  role: 'operator' | 'harness'
  body: string
}

/**
 * The conversation surface.
 *
 * Structured the way onyx structures its own — a scrolling message list above
 * a pinned composer — and built from the same Opal primitives, but over this
 * project's message shape rather than onyx's. Theirs is modelled on a RAG
 * product: `Message` carries retrieved documents, tool invocations, agent
 * identities and an `LlmManager`. Importing it would mean adopting that model
 * before there is anything behind it to fill in.
 */
export default function Conversation({ core }: { core: ComponentState | 'checking' }) {
  const [messages] = useState<ConversationMessage[]>([])
  const [draft, setDraft] = useState('')

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
        {messages.length === 0 ? (
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
            {messages.map((message) => (
              <li
                key={message.id}
                data-role={message.role}
                className={
                  message.role === 'operator'
                    ? 'self-end rounded-xl bg-background-neutral-02 px-4 py-2.5'
                    : 'self-start'
                }
              >
                <Text font="main-content-body" color="text-04">
                  {message.body}
                </Text>
              </li>
            ))}
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
