import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, test } from 'vitest'
import Conversation from './Conversation'

test('the composer refuses input and says why while the core is down', async () => {
  render(<Conversation core="unavailable" />)

  // The reason is stated in text, not only implied by a greyed-out field:
  // "why can I not type" must be answerable without guessing.
  expect(
    screen.getAllByText(/Harness Core is not running/).length,
  ).toBeGreaterThan(0)
  expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled()
})

test('the gate tracks the broker rather than a flag, and reads differently while checking', () => {
  render(<Conversation core="checking" />)
  // `checking` is the gap before the broker's first answer. Reporting it as a
  // failure would be a verdict the renderer has not earned yet.
  expect(screen.getAllByText(/Checking the runtime/).length).toBeGreaterThan(0)
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
