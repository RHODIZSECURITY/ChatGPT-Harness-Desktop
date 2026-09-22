import { render, screen } from '@testing-library/react'
import { expect, test } from 'vitest'
import { turn } from './fixtures'
import { groupPackets } from './grouping'
import { PacketType } from './protocol'
import type { PacketObject } from './protocol'
import {
  BashToolRenderer,
  CodingAgentRenderer,
  CustomToolRenderer,
  ErrorRenderer,
  FileReaderRenderer,
  MessageTextRenderer,
  ReasoningRenderer,
} from './renderers'
import type { RendererProps } from './renderers'

/** Render one renderer over the group these packets form. */
function show(Renderer: (props: RendererProps) => unknown, ...objs: PacketObject[]) {
  const [group] = groupPackets(objs.map((obj) => turn(0, obj)))
  const Component = Renderer as (props: RendererProps) => React.ReactElement
  render(<Component group={group!} />)
}

test('a shell verdict survives the deltas that arrive after it', () => {
  // Deltas report `exit_code: null` while the process is still running, and
  // a delta can follow the one that carried the code. Letting a later null
  // through would silently turn a failure into no verdict at all.
  show(
    BashToolRenderer,
    { type: PacketType.BASH_TOOL_START, cmd: 'false' },
    { type: PacketType.BASH_TOOL_DELTA, stdout: '', stderr: '', exit_code: 1, timed_out: false },
    { type: PacketType.BASH_TOOL_DELTA, stdout: '', stderr: '', exit_code: null, timed_out: false },
  )

  expect(screen.getByText('Exited 1')).toBeTruthy()
})

test('a timeout outranks the exit code it reports', () => {
  // A killed process exits with a code describing the signal, not the work.
  show(
    BashToolRenderer,
    { type: PacketType.BASH_TOOL_DELTA, stdout: '', stderr: '', exit_code: 137, timed_out: true },
  )

  expect(screen.getByText('Timed out')).toBeTruthy()
  expect(screen.queryByText('Exited 137')).toBeNull()
})

test('a shell that failed halfway keeps the output that explains it', () => {
  // The dispatcher does not route errors away from the work they interrupted.
  show(
    BashToolRenderer,
    { type: PacketType.BASH_TOOL_START, cmd: 'build' },
    { type: PacketType.BASH_TOOL_DELTA, stdout: 'compiling', stderr: '', exit_code: null, timed_out: false },
    { type: PacketType.ERROR, message: 'runtime lost' },
  )

  expect(screen.getByLabelText('Standard output').textContent).toBe('compiling')
  expect(screen.getByRole('alert').textContent).toContain('runtime lost')
})

test("a tool's own failure is surfaced, though it is not an error packet", () => {
  // It rides the delta, so `groupError` cannot see it. Without this the tool
  // would render as if it had simply produced nothing.
  show(
    CustomToolRenderer,
    {
      type: PacketType.CUSTOM_TOOL_DELTA,
      tool_name: 'fetch_manifest',
      response_type: 'json',
      error: { error_message: 'signature rejected' },
    },
  )

  expect(screen.getByRole('alert').textContent).toContain('signature rejected')
})

test('a partial read says so; a whole read does not', () => {
  const result = (start: number, end: number, total: number): PacketObject => ({
    type: PacketType.FILE_READER_RESULT,
    file_name: 'main.rs',
    file_id: 'f1',
    start_char: start,
    end_char: end,
    total_chars: total,
    preview_start: 'fn main',
    preview_end: '',
  })

  show(FileReaderRenderer, result(0, 400, 400))
  expect(screen.queryByText(/characters/)).toBeNull()

  show(FileReaderRenderer, result(120, 400, 400))
  expect(screen.getByText('characters 120–400 of 400')).toBeTruthy()
})

test("the coding agent's working notes outlive its answer", () => {
  // The notes are the record of what it did to the repository; the answer is
  // only a summary of it.
  show(
    CodingAgentRenderer,
    { type: PacketType.CODING_AGENT_START, query: 'fix the parser', repo: 'harness' },
    { type: PacketType.CODING_AGENT_THINKING_DELTA, content: 'reading lexer.rs' },
    { type: PacketType.CODING_AGENT_FINAL, answer: 'Patched the lexer.' },
  )

  expect(screen.getByLabelText('Coding agent thinking').textContent).toBe('reading lexer.rs')
  expect(screen.getByText('Patched the lexer.')).toBeTruthy()
})

test('a message renders its markdown as markup, not as characters', () => {
  show(MessageTextRenderer, { type: PacketType.MESSAGE_DELTA, content: '**provisioned**' })

  expect(screen.getByText('provisioned').tagName).toBe('STRONG')
})

test('reasoning says whether it is still going', () => {
  // Mid-stream and finished look identical otherwise, and a transcript that
  // cannot say which is which reads as if the model stalled.
  show(ReasoningRenderer, { type: PacketType.REASONING_DELTA, reasoning: 'weighing' })
  expect(screen.getByText('Thinking…')).toBeTruthy()

  show(
    ReasoningRenderer,
    { type: PacketType.REASONING_DELTA, reasoning: 'weighing' },
    { type: PacketType.REASONING_DONE },
  )
  expect(screen.getByText('Thought')).toBeTruthy()
  expect(screen.getAllByText('weighing')).toHaveLength(2)
})

test('a tool shows the arguments it was called with', () => {
  show(CustomToolRenderer, {
    type: PacketType.CUSTOM_TOOL_ARGS,
    tool_name: 'read_file',
    tool_args: { path: '/etc/wsl.conf' },
  })

  expect(screen.getByLabelText('Tool arguments').textContent).toContain('/etc/wsl.conf')
})

test('a turn that broke before identifying itself still reports the failure', () => {
  show(ErrorRenderer, { type: PacketType.ERROR, message: 'stream closed' })

  expect(screen.getByRole('alert').textContent).toContain('stream closed')
})
