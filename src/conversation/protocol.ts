/**
 * The streaming packet protocol.
 *
 * Adapted from onyx-foss `web/src/app/app/services/streamingModels.ts`
 * (MIT, commit e11c874019dbf04032cfc3476d82eba1d069a3d8). Not vendored: this
 * is a subset, and a subset of a file is a different file, so it is written
 * here rather than copied and trimmed — see PROVENANCE.md.
 *
 * The wire strings are kept byte-identical to onyx's. They are the protocol;
 * renaming them would buy nothing and would cost the ability to read onyx's
 * own backend, tests and traces as a reference for what a packet means.
 *
 * What is left out is the retrieval product: search, citations, deep research,
 * research agents, intermediate reports, image generation and the memory tool.
 * Those describe a RAG pipeline this application does not have. What is kept
 * is the part that describes an agent doing work — text, reasoning, a coding
 * agent, a shell, a file read, an arbitrary tool — which is exactly what the
 * Harness runs.
 */

/**
 * Where a packet belongs in the transcript.
 *
 * A stream is not a list: tools run in parallel and models answer side by
 * side, so position is a coordinate rather than an arrival order. Packets
 * sharing a `turn_index` but differing in `tab_index` ran concurrently.
 */
export interface Placement {
  turn_index: number
  /** Distinguishes tools that ran in parallel within one turn. */
  tab_index?: number
  sub_turn_index?: number | null
  /** Which model produced this packet, when several answer at once. */
  model_index?: number | null
}

export const PacketType = {
  MESSAGE_START: 'message_start',
  MESSAGE_DELTA: 'message_delta',
  MESSAGE_END: 'message_end',

  STOP: 'stop',
  SECTION_END: 'section_end',
  TOP_LEVEL_BRANCHING: 'top_level_branching',
  ERROR: 'error',

  REASONING_START: 'reasoning_start',
  REASONING_DELTA: 'reasoning_delta',
  REASONING_DONE: 'reasoning_done',

  CODING_AGENT_START: 'coding_agent_start',
  CODING_AGENT_THINKING_DELTA: 'coding_agent_thinking_delta',
  CODING_AGENT_FINAL: 'coding_agent_final',

  BASH_TOOL_START: 'bash_tool_start',
  BASH_TOOL_DELTA: 'bash_tool_delta',

  FILE_READER_START: 'file_reader_start',
  FILE_READER_RESULT: 'file_reader_result',

  CUSTOM_TOOL_START: 'custom_tool_start',
  CUSTOM_TOOL_ARGS: 'custom_tool_args',
  CUSTOM_TOOL_DELTA: 'custom_tool_delta',
} as const

export type PacketType = (typeof PacketType)[keyof typeof PacketType]

export const StopReason = {
  FINISHED: 'finished',
  USER_CANCELLED: 'user_cancelled',
} as const

export type StopReason = (typeof StopReason)[keyof typeof StopReason]

// --- Message ---------------------------------------------------------------

export interface MessageStart {
  type: typeof PacketType.MESSAGE_START
  id: string
  content: string
}

export interface MessageDelta {
  type: typeof PacketType.MESSAGE_DELTA
  content: string
}

export interface MessageEnd {
  type: typeof PacketType.MESSAGE_END
}

// --- Control ---------------------------------------------------------------

export interface Stop {
  type: typeof PacketType.STOP
  stop_reason?: StopReason
}

export interface SectionEnd {
  type: typeof PacketType.SECTION_END
}

export interface TopLevelBranching {
  type: typeof PacketType.TOP_LEVEL_BRANCHING
  num_parallel_branches: number
}

export interface PacketError {
  type: typeof PacketType.ERROR
  message?: string
}

// --- Reasoning -------------------------------------------------------------

export interface ReasoningStart {
  type: typeof PacketType.REASONING_START
}

export interface ReasoningDelta {
  type: typeof PacketType.REASONING_DELTA
  reasoning: string
}

export interface ReasoningDone {
  type: typeof PacketType.REASONING_DONE
}

// --- Coding agent ----------------------------------------------------------

export interface CodingAgentStart {
  type: typeof PacketType.CODING_AGENT_START
  query: string
  repo: string | null
}

export interface CodingAgentThinkingDelta {
  type: typeof PacketType.CODING_AGENT_THINKING_DELTA
  content: string
}

export interface CodingAgentFinal {
  type: typeof PacketType.CODING_AGENT_FINAL
  answer: string
}

// --- Shell -----------------------------------------------------------------

export interface BashToolStart {
  type: typeof PacketType.BASH_TOOL_START
  cmd: string
}

export interface BashToolDelta {
  type: typeof PacketType.BASH_TOOL_DELTA
  stdout: string
  stderr: string
  /** Null while the command is still running. */
  exit_code: number | null
  timed_out: boolean
}

// --- File reader -----------------------------------------------------------

export interface FileReaderStart {
  type: typeof PacketType.FILE_READER_START
}

export interface FileReaderResult {
  type: typeof PacketType.FILE_READER_RESULT
  file_name: string
  file_id: string
  start_char: number
  end_char: number
  total_chars: number
  preview_start: string
  preview_end: string
}

// --- Custom tool -----------------------------------------------------------

export interface CustomToolStart {
  type: typeof PacketType.CUSTOM_TOOL_START
  tool_name: string
  tool_id?: number | null
}

export interface CustomToolArgs {
  type: typeof PacketType.CUSTOM_TOOL_ARGS
  tool_name: string
  tool_args: Record<string, unknown>
}

export interface CustomToolDelta {
  type: typeof PacketType.CUSTOM_TOOL_DELTA
  tool_name: string
  tool_id?: number | null
  response_type: string
  data?: unknown
  error?: { error_message: string } | null
}

// --- Union -----------------------------------------------------------------

export type PacketObject =
  | MessageStart
  | MessageDelta
  | MessageEnd
  | Stop
  | SectionEnd
  | TopLevelBranching
  | PacketError
  | ReasoningStart
  | ReasoningDelta
  | ReasoningDone
  | CodingAgentStart
  | CodingAgentThinkingDelta
  | CodingAgentFinal
  | BashToolStart
  | BashToolDelta
  | FileReaderStart
  | FileReaderResult
  | CustomToolStart
  | CustomToolArgs
  | CustomToolDelta

export interface Packet {
  placement: Placement
  obj: PacketObject
}

/** Narrow a packet to the members of `types`, preserving the union member. */
export function isOneOf<T extends PacketObject['type']>(
  packet: Packet,
  types: readonly T[],
): packet is Packet & { obj: Extract<PacketObject, { type: T }> } {
  return (types as readonly string[]).includes(packet.obj.type)
}
