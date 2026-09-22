import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, test } from 'vitest'
import Conversation from './Conversation'

test('the composer refuses input and says why while the core is down', async () => {
  render(<Conversation core="unavailable" />)

  // The reason is stated in text, not only implied by a greyed-out field:
  // "why can I not type" must be answerable without guessing. Exactly once,
  // though — it was briefly rendered as both the placeholder and the note
  // below, which reads as a rendering fault rather than an explanation.
  expect(screen.getAllByText(/Harness Core is not running/)).toHaveLength(1)
  expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled()

  // And not as the placeholder as well. A placeholder is an attribute rather
  // than text content, so the assertion above cannot see it — the duplicate
  // was visible in the running window while the suite stayed green.
  expect(screen.getByLabelText('Message')).toHaveAttribute(
    'placeholder',
    'Ask the Harness…',
  )
})

test('the gate tracks the broker rather than a flag, and reads differently while checking', () => {
  render(<Conversation core="checking" />)
  // `checking` is the gap before the broker's first answer. Reporting it as a
  // failure would be a verdict the renderer has not earned yet.
  expect(screen.getAllByText(/Checking the runtime/)).toHaveLength(1)
  expect(screen.queryByText(/not running/)).not.toBeInTheDocument()
})

test('Send stays disabled on whitespace once the core is ready', async () => {
  const user = userEvent.setup()
  render(<Conversation core="ready" />)

  const send = screen.getByRole('button', { name: 'Send' })
  expect(send).toBeDisabled()

  await user.type(screen.getByLabelText('Message'), '   ')
  expect(send, 'whitespace is not a message').toBeDisabled()

  await user.type(screen.getByLabelText('Message'), 'provision the runtime')
  expect(send).toBeEnabled()
})

test('the empty state is a statement, not an error', () => {
  render(<Conversation core="ready" />)
  expect(screen.getByText('Nothing here yet')).toBeInTheDocument()
})

test('the transcript renders the packets it is given, and skips the ones with nothing to show', () => {
  render(
    <Conversation
      core="ready"
      packets={[
        { placement: { turn_index: 0 }, obj: { type: 'message_delta', content: 'Provisioned.' } },
        { placement: { turn_index: 1 }, obj: { type: 'stop', stop_reason: 'finished' } },
      ]}
    />,
  )

  expect(screen.getByText('Provisioned.')).toBeTruthy()
  // The `stop` group carries no content, and an empty list item would still
  // take up a row in the transcript.
  expect(screen.getAllByRole('listitem')).toHaveLength(1)
  expect(screen.queryByText('Nothing here yet')).toBeNull()
})
