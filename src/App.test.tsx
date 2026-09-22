import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, test, vi } from 'vitest'

vi.mock('./runtime/bridge', () => ({ getRuntimeStatus: vi.fn() }))

import App from './App'
import { SIDEBAR_FOLDED_KEY, WORKSPACE_OPEN_KEY } from './shell/panelState'
import { getRuntimeStatus } from './runtime/bridge'
import type { RuntimeStatus } from './runtime/types'

const ready: RuntimeStatus = {
  platform: 'windows',
  wsl2: { state: 'ready' },
  docker: { state: 'ready' },
  core: { state: 'ready' },
  route: { state: 'ready' },
  memory: { state: 'ready' },
  providers: { state: 'ready' },
}

beforeEach(() => {
  vi.clearAllMocks()
  // Both panels persist their state, so a test that did not clear this would
  // be starting from whatever the previous one closed.
  localStorage.clear()
})
test('renders the three desktop work areas and live runtime states', async () => {
  vi.mocked(getRuntimeStatus).mockResolvedValue(ready)
  render(<App />)

  // By role, not by text: Opal's Text renders a span unless told otherwise,
  // so this is what keeps the workspace title reachable by landmark rather
  // than merely looking like a title.
  expect(screen.getByRole('heading', { name: 'Harness workspace' })).toBeInTheDocument()

  // The sidebar and workspace rows are the design system's own disabled
  // controls now, not spans dimmed by hand, so unavailability is announced
  // rather than merely drawn.
  //
  // Asserted as aria-disabled and not as a disabled button on purpose: Opal
  // renders these as a div carrying `aria-disabled`, with no `role`. That is
  // weaker than a real control — a screen reader will not call it a button —
  // but it is what the library exposes, and a test that claimed otherwise
  // would be describing a component this project does not have.
  for (const label of ['Projects', 'Sessions', 'Files', 'Terminal']) {
    const row = screen.getByText(label).closest('[aria-disabled]')
    expect(row, `${label} is not rendered as a disabled control`).not.toBeNull()
    expect(row).toHaveAttribute('aria-disabled', 'true')
  }
  expect(screen.getByLabelText('Projects and sessions')).toBeInTheDocument()
  expect(screen.getByLabelText('Conversation')).toBeInTheDocument()
  expect(screen.getByLabelText('Workspace details')).toBeInTheDocument()
  expect(await screen.findByText('Broker: windows')).toBeInTheDocument()
  expect(screen.getByText('RHODIZ MCP Route: ready')).toBeInTheDocument()
  expect(screen.getByText('RHODIZ Memory MCP: ready')).toBeInTheDocument()
})

test('fails closed in the UI when the broker cannot be reached', async () => {
  vi.mocked(getRuntimeStatus).mockRejectedValue(new Error('offline'))
  render(<App />)
  expect(await screen.findByText('Broker unavailable')).toBeInTheDocument()
  expect(screen.getByText('Harness Core: checking')).toBeInTheDocument()
})

test('both panels can be put away, and each remembers it', async () => {
  vi.mocked(getRuntimeStatus).mockResolvedValue(ready)
  const user = userEvent.setup()
  render(<App />)

  // Closed is width zero and `inert`, not unmounted: the panel keeps its
  // scroll position across a close, and `display: none` cannot be animated.
  // Zero is written as `0px` because that is how the style is read back — what
  // is asserted is that the shell asked for none of the width, not that a
  // browser without layout gave it none.
  const panel = screen.getByLabelText('Workspace details').parentElement
  expect(panel).not.toHaveAttribute('inert')
  expect(panel).not.toHaveStyle({ width: '0px' })
  await user.click(screen.getByRole('button', { name: 'Hide workspace panel' }))
  expect(panel).toHaveAttribute('inert')
  expect(panel).toHaveStyle({ width: '0px' })
  // The control has to keep naming the way out, or closing the panel would
  // hide the only thing that reopens it.
  expect(screen.getByRole('button', { name: 'Show workspace panel' })).toBeInTheDocument()
  expect(localStorage.getItem(WORKSPACE_OPEN_KEY)).toBe('false')

  // The sidebar folds through Opal's own control. Asserted on the wordmark
  // rather than on a width: the product name is the one thing in the header
  // that cannot fit a 3.25rem rail, so its absence is what folded means.
  expect(screen.getByText('RHODIZ Harness')).toBeInTheDocument()
  await user.click(screen.getByRole('button', { name: 'Close Sidebar' }))
  expect(screen.queryByText('RHODIZ Harness')).not.toBeInTheDocument()
  expect(localStorage.getItem(SIDEBAR_FOLDED_KEY)).toBe('true')
})

test('opens with both panels as the last session left them', async () => {
  vi.mocked(getRuntimeStatus).mockResolvedValue(ready)
  localStorage.setItem(WORKSPACE_OPEN_KEY, 'false')
  localStorage.setItem(SIDEBAR_FOLDED_KEY, 'true')
  render(<App />)

  // Read on the first render, not in an effect: a panel that starts open and
  // animates itself shut is a panel that ignored the preference for 200ms in
  // full view.
  expect(screen.getByLabelText('Workspace details').parentElement).toHaveAttribute('inert')
  expect(screen.queryByText('RHODIZ Harness')).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Open Sidebar' })).toBeInTheDocument()
})
