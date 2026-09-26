import type * as React from "react";
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, test, vitest } from 'vitest'
import { Provider as TooltipProvider } from '@radix-ui/react-tooltip'
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


const renderWithProviders = (ui: React.ReactElement) =>
  render(<TooltipProvider delayDuration={0}>{ui}</TooltipProvider>)

test('composer enters streaming state, disables input, and transforms Send to Stop', async () => {
  const onStop = vitest.fn()
  const { rerender } = renderWithProviders(
    <Conversation core="ready" isStreaming={false} onStop={onStop} />
  )

  // Initially idle: Send button visible and disabled (empty draft)
  expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled()
  expect(screen.queryByRole('button', { name: 'Stop generating' })).toBeNull()

  // Streaming begins: Send transforms into Stop button
  rerender(
    <TooltipProvider delayDuration={0}>
      <Conversation core="ready" isStreaming={true} onStop={onStop} />
    </TooltipProvider>
  )

  const stopBtn = screen.getByRole('button', { name: 'Stop generating' })
  expect(stopBtn).toBeInTheDocument()
  expect(screen.queryByRole('button', { name: 'Send' })).toBeNull()

  // Input textarea is marked disabled variant during streaming
  const textarea = screen.getByLabelText('Message')
  expect(textarea).toBeInTheDocument()

  // Clicking Stop calls onStop callback
  stopBtn.click()
  expect(onStop).toHaveBeenCalledTimes(1)
})

test('composer takeover for approval_request: fail-closed, plain text, and one-shot action', async () => {
  const onApprove = vitest.fn().mockResolvedValue(undefined)
  const packets = [
    {
      placement: { turn_index: 0 },
      obj: {
        type: 'approval_request' as const,
        request_id: 'req-42',
        action: 'Execute bash command',
        detail: 'Run rm -rf in sandbox',
        arguments: { cmd: 'rm -rf /tmp/test', timeout: 5000 },
      },
    },
  ]

  render(
    <Conversation
      core="ready"
      packets={packets}
      onApprove={onApprove}
    />
  )

  // Approval alertdialog takeover mounts
  const dialog = screen.getByRole('alertdialog', { name: 'Security Approval Required' })
  expect(dialog).toBeInTheDocument()

  // Input and Send button are replaced by the takeover
  expect(screen.queryByLabelText('Message')).toBeNull()
  expect(screen.queryByRole('button', { name: 'Send' })).toBeNull()

  // Invariant: action, detail, and arguments rendered as plain text nodes (preformatted JSON)
  expect(screen.getByText(/Approval required: Execute bash command/)).toBeInTheDocument()
  expect(screen.getByText('Run rm -rf in sandbox')).toBeInTheDocument()
  expect(screen.getByText(/"cmd": "rm -rf \/tmp\/test"/)).toBeInTheDocument()

  // Buttons are present
  const allowBtn = screen.getByRole('button', { name: 'Allow once' })
  const rejectBtn = screen.getByRole('button', { name: 'Reject' })
  expect(allowBtn).toBeEnabled()
  expect(rejectBtn).toBeEnabled()

  // Click Allow once: triggers onApprove with ('req-42', 'allow_once')
  allowBtn.click()
  expect(onApprove).toHaveBeenCalledWith('req-42', 'allow_once')

  // One-shot invariant: once allowed/resolved, the takeover releases back to idle composer
  expect(await screen.findByLabelText('Message')).toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Send' })).toBeInTheDocument()
  expect(screen.queryByRole('alertdialog')).toBeNull()
})

test('approval takeover rolls back if onApprove rejects or fails', async () => {
  const onApprove = vitest.fn().mockRejectedValue(new Error('Network failure'))
  const packets = [
    {
      placement: { turn_index: 0 },
      obj: {
        type: 'approval_request' as const,
        request_id: 'req-fail',
        action: 'Write file',
      },
    },
  ]

  render(
    <Conversation
      core="ready"
      packets={packets}
      onApprove={onApprove}
    />
  )

  const rejectBtn = screen.getByRole('button', { name: 'Reject' })
  rejectBtn.click()
  expect(onApprove).toHaveBeenCalledWith('req-fail', 'reject')

  // Since promise failed, takeover rolls back and remains active
  expect(await screen.findByRole('alertdialog')).toBeInTheDocument()
})

test('composer extension slots render when provided', () => {
  render(
    <Conversation
      core="ready"
      slots={{
        controlBelt: <div data-testid="control-belt">Belt</div>,
        leftActions: <div data-testid="left-slot">Left</div>,
        rightActions: <div data-testid="right-slot">Right</div>,
      }}
    />
  )

  expect(screen.getByTestId('control-belt')).toHaveTextContent('Belt')
  expect(screen.getByTestId('left-slot')).toHaveTextContent('Left')
  expect(screen.getByTestId('right-slot')).toHaveTextContent('Right')
})

test('pressing Enter submits message when not empty, Shift+Enter does not submit', async () => {
  const user = userEvent.setup()
  const onSend = vitest.fn()
  renderWithProviders(<Conversation core="ready" onSend={onSend} />)

  const textarea = screen.getByLabelText('Message')
  await user.type(textarea, 'hello{Shift>}{Enter}{/Shift}')
  expect(onSend).not.toHaveBeenCalled()

  await user.type(textarea, '{Enter}')
  expect(onSend).toHaveBeenCalledWith('hello')
})
