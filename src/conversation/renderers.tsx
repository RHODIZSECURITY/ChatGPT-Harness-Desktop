import { CompactMarkdown, Text } from '@opal/components'
import {
  type PacketGroup,
  groupError,
  reasoningContent,
  textContent,
} from './grouping'
import { type Packet, PacketType, isOneOf } from './protocol'

export interface RendererProps {
  group: PacketGroup
}

/**
 * The failure a group carries, rendered inside the work it interrupted.
 *
 * Every renderer ends with this rather than the dispatcher routing errors to
 * a renderer of their own: a shell command that failed halfway is still a
 * shell command, and showing only the error would throw away the output that
 * explains it.
 */
function GroupError({ packets }: { packets: Packet[] }) {
  const message = groupError(packets)
  if (message === null) return null
  return (
    <div role="alert" className="mt-2">
      <Text font="secondary-body" color="status-error-05">
        {message}
      </Text>
    </div>
  )
}

/** A tool's header line: what ran, and on what. */
function ToolHeader({ label, detail }: { label: string; detail?: string }) {
  return (
    <div className="flex items-baseline gap-2">
      <Text font="main-ui-muted" color="text-03">
        {label}
      </Text>
      {detail !== undefined && detail !== '' && (
        <span className="min-w-0 truncate font-mono text-xs text-text-02">{detail}</span>
      )}
    </div>
  )
}

/** Monospaced output, preserved as the process wrote it. */
function Stream({ label, body }: { label: string; body: string }) {
  if (body === '') return null
  return (
    <pre
      aria-label={label}
      className="mt-2 max-h-80 overflow-auto rounded-lg bg-background-neutral-02 px-3 py-2 font-mono text-xs whitespace-pre-wrap text-text-04"
    >
      {body}
    </pre>
  )
}

export function MessageTextRenderer({ group }: RendererProps) {
  const text = textContent(group.packets)
  return (
    <div data-renderer="message">
      {text !== '' && <CompactMarkdown>{text}</CompactMarkdown>}
      <GroupError packets={group.packets} />
    </div>
  )
}

export function ReasoningRenderer({ group }: RendererProps) {
  const reasoning = reasoningContent(group.packets)
  const done = group.packets.some(
    (packet) => packet.obj.type === PacketType.REASONING_DONE,
  )
  return (
    <div data-renderer="reasoning" className="border-l border-border-01 pl-3">
      <Text font="main-ui-muted" color="text-03">
        {done ? 'Thought' : 'Thinking…'}
      </Text>
      {reasoning !== '' && (
        <div className="mt-1">
          <Text font="secondary-body" color="text-02">
            {reasoning}
          </Text>
        </div>
      )}
      <GroupError packets={group.packets} />
    </div>
  )
}

export function CodingAgentRenderer({ group }: RendererProps) {
  let query = ''
  let repo: string | null = null
  let thinking = ''
  let answer: string | null = null

  for (const packet of group.packets) {
    if (isOneOf(packet, [PacketType.CODING_AGENT_START])) {
      query = packet.obj.query
      repo = packet.obj.repo
    } else if (isOneOf(packet, [PacketType.CODING_AGENT_THINKING_DELTA])) {
      thinking += packet.obj.content
    } else if (isOneOf(packet, [PacketType.CODING_AGENT_FINAL])) {
      answer = packet.obj.answer
    }
  }

  return (
    <div data-renderer="coding-agent" className="rounded-lg border border-border-01 p-3">
      <ToolHeader label="Coding agent" detail={repo ?? undefined} />
      {query !== '' && (
        <div className="mt-1">
          <Text font="secondary-body" color="text-03">
            {query}
          </Text>
        </div>
      )}
      {/* The thinking stays visible after the answer arrives. It is the record
          of what the agent did to the repository, which outlives the summary
          of it. */}
      <Stream label="Coding agent thinking" body={thinking} />
      {answer !== null && (
        <div className="mt-2">
          <CompactMarkdown>{answer}</CompactMarkdown>
        </div>
      )}
      <GroupError packets={group.packets} />
    </div>
  )
}

export function BashToolRenderer({ group }: RendererProps) {
  let cmd = ''
  let stdout = ''
  let stderr = ''
  let exitCode: number | null = null
  let timedOut = false

  for (const packet of group.packets) {
    if (isOneOf(packet, [PacketType.BASH_TOOL_START])) {
      cmd = packet.obj.cmd
    } else if (isOneOf(packet, [PacketType.BASH_TOOL_DELTA])) {
      stdout += packet.obj.stdout
      stderr += packet.obj.stderr
      // The last delta carries the verdict; earlier ones report null while
      // the process is still running, so a later null must not erase it.
      if (packet.obj.exit_code !== null) exitCode = packet.obj.exit_code
      if (packet.obj.timed_out) timedOut = true
    }
  }

  return (
    <div data-renderer="bash" className="rounded-lg border border-border-01 p-3">
      <ToolHeader label="Shell" detail={cmd} />
      <Stream label="Standard output" body={stdout} />
      <Stream label="Standard error" body={stderr} />
      {/* A non-zero exit is the whole result of a command that printed
          nothing, so it is stated rather than left to be inferred from empty
          output. `timed_out` outranks it: a killed process reports an exit
          code that describes the signal, not the work. */}
      {(timedOut || exitCode !== null) && (
        <div className="mt-2">
          <Text
            font="secondary-body"
            color={timedOut || exitCode !== 0 ? 'status-error-05' : 'text-02'}
          >
            {timedOut ? 'Timed out' : `Exited ${exitCode}`}
          </Text>
        </div>
      )}
      <GroupError packets={group.packets} />
    </div>
  )
}

export function FileReaderRenderer({ group }: RendererProps) {
  const results = group.packets.filter((packet) =>
    isOneOf(packet, [PacketType.FILE_READER_RESULT]),
  )

  return (
    <div data-renderer="file-reader" className="rounded-lg border border-border-01 p-3">
      <ToolHeader label="Read" />
      {results.map((packet, index) => {
        if (!isOneOf(packet, [PacketType.FILE_READER_RESULT])) return null
        const { file_name, file_id, start_char, end_char, total_chars } = packet.obj
        // The range is shown because a preview of the middle of a file is a
        // different claim from a preview of the whole of it.
        const partial = start_char > 0 || end_char < total_chars
        return (
          <div key={`${file_id}-${index}`} className="mt-2">
            <div className="flex items-baseline gap-2">
              <span className="font-mono text-xs text-text-04">{file_name}</span>
              {partial && (
                <Text font="secondary-body" color="text-02">
                  {`characters ${start_char}–${end_char} of ${total_chars}`}
                </Text>
              )}
            </div>
            <Stream
              label={`Preview of ${file_name}`}
              body={
                packet.obj.preview_end === ''
                  ? packet.obj.preview_start
                  : `${packet.obj.preview_start}\n…\n${packet.obj.preview_end}`
              }
            />
          </div>
        )
      })}
      <GroupError packets={group.packets} />
    </div>
  )
}

export function CustomToolRenderer({ group }: RendererProps) {
  let toolName = ''
  let args: Record<string, unknown> | null = null
  let failure: string | null = null

  for (const packet of group.packets) {
    if (
      isOneOf(packet, [
        PacketType.CUSTOM_TOOL_START,
        PacketType.CUSTOM_TOOL_ARGS,
        PacketType.CUSTOM_TOOL_DELTA,
      ])
    ) {
      toolName = packet.obj.tool_name
    }
    if (isOneOf(packet, [PacketType.CUSTOM_TOOL_ARGS])) {
      args = packet.obj.tool_args
    }
    // A tool's own error is carried on its delta rather than as an `error`
    // packet, so `GroupError` would never see it.
    if (isOneOf(packet, [PacketType.CUSTOM_TOOL_DELTA])) {
      const error = packet.obj.error
      if (error !== null && error !== undefined) failure = error.error_message
    }
  }

  return (
    <div data-renderer="custom-tool" className="rounded-lg border border-border-01 p-3">
      <ToolHeader label="Tool" detail={toolName} />
      {args !== null && <Stream label="Tool arguments" body={JSON.stringify(args, null, 2)} />}
      {failure !== null && (
        <div role="alert" className="mt-2">
          <Text font="secondary-body" color="status-error-05">
            {failure}
          </Text>
        </div>
      )}
      <GroupError packets={group.packets} />
    </div>
  )
}

/**
 * A group whose only content is a failure — the turn broke before any tool or
 * message identified itself.
 */
export function ErrorRenderer({ group }: RendererProps) {
  return (
    <div data-renderer="error">
      <GroupError packets={group.packets} />
    </div>
  )
}
