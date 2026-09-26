import { memo, useCallback, useEffect, useMemo, useState } from 'react'
import { SidebarTab, Text } from '@opal/components'
import { SvgHistory, SvgBubbleText, SvgFolder, SvgPlus } from '@opal/icons'
import { SidebarLayouts, useSidebarFolded } from '@opal/layouts'
import {
  ACTIVE_SESSION_STORAGE_KEY,
  MAX_SESSIONS,
  SESSIONS_STORAGE_KEY,
  createNewSession,
  loadActiveSessionId,
  loadSessions,
  sanitizeTitle,
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

const renderAppLogo = () => RhodizMark

function Wordmark() {
  const folded = useSidebarFolded()
  if (folded) return null
  return (
    <Text font="main-ui-action" color="text-04" wordWrap="whitespace-nowrap">
      RHODIZ Harness
    </Text>
  )
}

interface SessionTabRowProps {
  session: SessionItem
  selected: boolean
  isArchived?: boolean
  onSelect: (id: string) => void
}

const SessionTabRow = memo(function SessionTabRow({
  session,
  selected,
  isArchived = false,
  onSelect,
}: SessionTabRowProps) {
  const handleClick = useCallback(() => {
    onSelect(session.id)
  }, [onSelect, session.id])

  const safeTitle = sanitizeTitle(session.title) || 'Untitled Session'
  const tooltipText = isArchived ? `${safeTitle} (archived)` : safeTitle
  const icon = isArchived ? SvgHistory : SvgBubbleText

  return (
    <SidebarTab
      icon={icon}
      selected={selected}
      onClick={handleClick}
      tooltip={tooltipText}
    >
      {safeTitle}
    </SidebarTab>
  )
})

export default function HarnessSidebar({
  activeSessionId: controlledActiveId,
  onSelectSession,
  onNewSession,
}: HarnessSidebarProps = {}) {
  const [sessions, setSessions] = useState<SessionItem[]>(() => loadSessions())
  const validIds = useMemo(() => sessions.map((s) => s.id), [sessions])
  const [internalActiveId, setInternalActiveId] = useState<string>(() => loadActiveSessionId(validIds))

  // Cross-window / cross-tab reactive synchronization
  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === SESSIONS_STORAGE_KEY) {
        const next = loadSessions()
        setSessions(next)
        setInternalActiveId(loadActiveSessionId(next.map((s) => s.id)))
      } else if (e.key === ACTIVE_SESSION_STORAGE_KEY) {
        setInternalActiveId(loadActiveSessionId(validIds))
      }
    }
    window.addEventListener('storage', handleStorage)
    return () => window.removeEventListener('storage', handleStorage)
  }, [validIds])

  // Ground activeId in validIds to prevent orphaned selections
  const activeId = useMemo(() => {
    const candidate = controlledActiveId ?? internalActiveId
    if (validIds.length > 0 && !validIds.includes(candidate)) {
      return validIds[0]
    }
    return candidate
  }, [controlledActiveId, internalActiveId, validIds])

  const handleSelect = useCallback(
    (id: string) => {
      if (!validIds.includes(id)) return
      setInternalActiveId(id)
      saveActiveSessionId(id, validIds)
      onSelectSession?.(id)
    },
    [validIds, onSelectSession],
  )

  const handleCreate = useCallback(() => {
    const session = createNewSession()
    // Read fresh from storage to mitigate lost update race condition
    const currentStored = loadSessions()
    const merged = [session, ...currentStored.filter((s) => s.id !== session.id)].slice(0, MAX_SESSIONS)
    const nextValidIds = merged.map((s) => s.id)

    setSessions(merged)
    saveSessions(merged)
    setInternalActiveId(session.id)
    saveActiveSessionId(session.id, nextValidIds)
    onNewSession?.(session)
    onSelectSession?.(session.id)
  }, [onNewSession, onSelectSession])

  const { activeSessions, archivedSessions } = useMemo(() => {
    const active: SessionItem[] = []
    const archived: SessionItem[] = []
    for (const item of sessions) {
      if (item.state === 'active') active.push(item)
      else if (item.state === 'archived') archived.push(item)
    }
    return { activeSessions: active, archivedSessions: archived }
  }, [sessions])

  return (
    <SidebarLayouts.Root foldable>
      <SidebarLayouts.Header renderAppLogo={renderAppLogo} showLogoWhenFolded>
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
              <SessionTabRow
                key={session.id}
                session={session}
                selected={session.id === activeId}
                onSelect={handleSelect}
              />
            ))}
          </SidebarLayouts.Section>
        </div>

        {/* Archived Sessions List (if any) */}
        {archivedSessions.length > 0 && (
          <div aria-label="Archived Sessions" className="mt-4">
            <SidebarLayouts.Section title="Archived">
              {archivedSessions.map((session) => (
                <SessionTabRow
                  key={session.id}
                  session={session}
                  selected={session.id === activeId}
                  isArchived
                  onSelect={handleSelect}
                />
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
