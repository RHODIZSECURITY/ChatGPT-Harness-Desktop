import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Provider as TooltipProvider } from '@radix-ui/react-tooltip'
import { beforeEach, expect, test, vitest } from 'vitest'
import { SidebarStateProvider } from '@opal/layouts'
import { RouterProvider } from '../design/next-shim/navigation'
import { ThemeProvider } from '../theme/ThemeProvider'
import HarnessSidebar from './HarnessSidebar'
import {
  DEFAULT_SESSION,
  saveSessions,
} from '../session/sessionStore'

function renderSidebar(props = {}) {
  return render(
    <ThemeProvider>
      <TooltipProvider delayDuration={0}>
        <RouterProvider>
          <SidebarStateProvider defaultFolded={false}>
            <HarnessSidebar {...props} />
          </SidebarStateProvider>
        </RouterProvider>
      </TooltipProvider>
    </ThemeProvider>,
  )
}

beforeEach(() => {
  localStorage.clear()
})

test('renders New Session button and default session in active list', () => {
  renderSidebar()

  expect(screen.getByRole('button', { name: 'New Session' })).toBeInTheDocument()
  expect(screen.getByText(DEFAULT_SESSION.title)).toBeInTheDocument()
  expect(screen.getByLabelText('Sessions')).toBeInTheDocument()
})

test('clicking New Session creates a new session and invokes callback', async () => {
  const user = userEvent.setup()
  const onNewSession = vitest.fn()
  renderSidebar({ onNewSession })

  const newSessionBtn = screen.getByRole('button', { name: 'New Session' })
  await user.click(newSessionBtn)

  expect(onNewSession).toHaveBeenCalledTimes(1)
  expect(onNewSession).toHaveBeenCalledWith(
    expect.objectContaining({
      title: 'New Session',
      state: 'active',
    }),
  )

  // Newly created session is listed in the sidebar
  expect(screen.getAllByText('New Session')).toHaveLength(2) // 1 in button, 1 in list
})

test('clicking a session item selects it and calls onSelectSession', async () => {
  const user = userEvent.setup()
  const onSelectSession = vitest.fn()

  saveSessions([
    { id: 's1', title: 'Refactor Auth', state: 'active', createdAt: '2026-09-26T10:00:00Z' },
    { id: 's2', title: 'Fix CSS Bug', state: 'active', createdAt: '2026-09-26T11:00:00Z' },
  ])

  renderSidebar({ onSelectSession, activeSessionId: 's1' })

  const session2Btn = screen.getByRole('button', { name: 'Fix CSS Bug' })
  await user.click(session2Btn)

  expect(onSelectSession).toHaveBeenCalledWith('s2')
})

test('renders archived section when archived sessions exist', () => {
  saveSessions([
    { id: 's1', title: 'Active Item', state: 'active', createdAt: '2026-09-26T10:00:00Z' },
    { id: 's2', title: 'Old History Item', state: 'archived', createdAt: '2026-09-20T10:00:00Z' },
  ])

  renderSidebar()

  expect(screen.getByLabelText('Archived Sessions')).toBeInTheDocument()
  expect(screen.getByText('Old History Item')).toBeInTheDocument()
})

test('New Session in controlled mode calls onSelectSession with new ID', async () => {
  const user = userEvent.setup()
  const onSelectSession = vitest.fn()
  const onNewSession = vitest.fn()

  renderSidebar({
    activeSessionId: 'initial-id',
    onSelectSession,
    onNewSession,
  })

  const newSessionBtn = screen.getByRole('button', { name: 'New Session' })
  await user.click(newSessionBtn)

  expect(onNewSession).toHaveBeenCalledTimes(1)
  expect(onSelectSession).toHaveBeenCalledTimes(1)
  const createdId = onNewSession.mock.calls[0][0].id
  expect(onSelectSession).toHaveBeenCalledWith(createdId)
})
