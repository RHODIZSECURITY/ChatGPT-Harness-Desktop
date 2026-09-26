import { useCallback, useState } from 'react'
import { SidebarTab, Text } from '@opal/components'
import { SvgHistory, SvgBubbleText, SvgFolder, SvgPlus } from '@opal/icons'
import { SidebarLayouts, useSidebarFolded } from '@opal/layouts'
import {
  createNewSession,
  loadActiveSessionId,
  loadSessions,
  saveActiveSessionId,
  saveSessions,
} from '../session/sessionStore'
import type { SessionItem } from '../session/types'
import RhodizMark from './RhodizMark'
import ThemeControl from './ThemeControl'

interface HarnessSidebarProps {
  activeSessionId?: string
  onSelectSession?: (id: string) => void
  onNewSession?: (session: SessionItem) => void
}

function Wordmark() {
  const folded = useSidebarFolded()
  if (folded) return null
  return (
    <Text font="main-ui-action" color="text-04" wordWrap="whitespace-nowrap">
      RHODIZ Harness
    </Text>
  )
}

export default function HarnessSidebar({
  activeSessionId: controlledActiveId,
  onSelectSession,
  onNewSession,
}: HarnessSidebarProps = {}) {
  const [sessions, setSessions] = useState<SessionItem[]>(() => loadSessions())
  const [internalActiveId, setInternalActiveId] = useState<string>(() => loadActiveSessionId())

  const activeId = controlledActiveId ?? internalActiveId

  const handleSelect = useCallback(
    (id: string) => {
      setInternalActiveId(id)
      saveActiveSessionId(id)
      onSelectSession?.(id)
    },
    [onSelectSession],
  )

  const handleCreate = useCallback(() => {
    const session = createNewSession()
    const nextSessions = [session, ...sessions]
    setSessions(nextSessions)
    saveSessions(nextSessions)
    setInternalActiveId(session.id)
    saveActiveSessionId(session.id)
    onNewSession?.(session)
  }, [sessions, onNewSession])

  const activeSessions = sessions.filter((s) => s.state === 'active')
  const archivedSessions = sessions.filter((s) => s.state === 'archived')

  return (
    <SidebarLayouts.Root foldable>
      <SidebarLayouts.Header renderAppLogo={() => RhodizMark} showLogoWhenFolded>
        <Wordmark />
      </SidebarLayouts.Header>

      <SidebarLayouts.Body scrollKey="harness-sidebar">
        {/* New Session Action */}
        <div className="px-1 pb-2">
          <SidebarTab
            icon={SvgPlus}
            onClick={handleCreate}
            aria-label="New Session"
            tooltip="New Session"
          >
            New Session
          </SidebarTab>
        </div>

        {/* Active Sessions List */}
        <div aria-label="Sessions">
          <SidebarLayouts.Section title="Sessions">
            {activeSessions.map((session) => (
              <SidebarTab
                key={session.id}
                icon={SvgBubbleText}
                selected={session.id === activeId}
                onClick={() => handleSelect(session.id)}
                tooltip={session.title}
              >
                {session.title}
              </SidebarTab>
            ))}
          </SidebarLayouts.Section>
        </div>

        {/* Archived Sessions List (if any) */}
        {archivedSessions.length > 0 && (
          <div aria-label="Archived Sessions" className="mt-4">
            <SidebarLayouts.Section title="Archived">
              {archivedSessions.map((session) => (
                <SidebarTab
                  key={session.id}
                  icon={SvgHistory}
                  selected={session.id === activeId}
                  onClick={() => handleSelect(session.id)}
                  tooltip={`${session.title} (archived)`}
                >
                  {session.title}
                </SidebarTab>
              ))}
            </SidebarLayouts.Section>
          </div>
        )}

        {/* Projects (Future Slot) */}
        <div aria-label="Projects" className="mt-4">
          <SidebarLayouts.Section title="Projects" disabled>
            <SidebarTab icon={SvgFolder} disabled>
              Projects
            </SidebarTab>
          </SidebarLayouts.Section>
        </div>
      </SidebarLayouts.Body>

      <SidebarLayouts.Footer>
        <div className="pb-3">
          <ThemeControl />
        </div>
      </SidebarLayouts.Footer>
    </SidebarLayouts.Root>
  )
}
