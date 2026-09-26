import { InputSingleSelect } from '@opal/components'
import type { RuntimeStatus } from '../runtime/types'
import { STATE_TONE } from './statusTone'

/**
 * The live runtime context the composer decides against.
 *
 * Short labels (`Core`, not `Harness Core`): the belt sits above the
 * composer, not in the window footer, so it names the same four components
 * the footer already states in full. Same tone map, same `data-state`
 * contract — two strips describing one runtime must not disagree.
 */
const CONTEXT_COMPONENTS: Array<[keyof Omit<RuntimeStatus, 'platform'>, string]> = [
  ['core', 'Core'],
  ['route', 'Route'],
  ['memory', 'Memory'],
  ['providers', 'Providers'],
]

export interface ComposerControlBeltProps {
  status: RuntimeStatus | null
  /** Options Core exposes for selection. Empty today: Core does not publish
      a model or role list over `runtime_status`, so the selects stay disabled
      on an honest placeholder rather than inventing choices. */
  models?: string[]
  roles?: string[]
  selectedModel?: string | null
  selectedRole?: string | null
  onModelChange?: (model: string) => void
  onRoleChange?: (role: string) => void
  /** Set while the composer is busy (streaming, approval takeover). Wired by
      the parent once it knows the composer state; today App does not track
      it, so the selects enable only when Core exposes options. */
  disabled?: boolean
}

interface BeltSelectProps {
  label: string
  placeholder: string
  title: string
  options: string[]
  value: string | null | undefined
  onChange?: (next: string) => void
  disabled: boolean
}

/**
 * One Core-owned selector.
 *
 * The value is anchored to the exposed list: a selection Core never offered
 * (stale storage, a forged prop) falls back to the placeholder instead of
 * displaying a value nothing will honour. Same on the way out — only listed
 * values reach the callback.
 */
function BeltSelect({
  label,
  placeholder,
  title,
  options,
  value,
  onChange,
  disabled,
}: BeltSelectProps) {
  // Empty string, not `undefined`, when nothing is anchored: Radix mirrors
  // `value` onto a hidden native select, which warns on an uncontrolled to
  // controlled switch. `''` matches no Item, so the placeholder still shows.
  const safeValue = value && options.includes(value) ? value : ''
  return (
    <div className="w-36" title={title}>
      <InputSingleSelect
        value={safeValue}
        disabled={disabled || options.length === 0}
        onValueChange={(next) => {
          if (options.includes(next)) onChange?.(next)
        }}
      >
        <InputSingleSelect.Trigger aria-label={label} placeholder={placeholder} />
        <InputSingleSelect.Content>
          {options.map((option) => (
            <InputSingleSelect.Item key={option} value={option}>
              {option}
            </InputSingleSelect.Item>
          ))}
        </InputSingleSelect.Content>
      </InputSingleSelect>
    </div>
  )
}

/**
 * The composer's control belt: model/role selectors plus live runtime
 * context indicators, rendered into `Conversation`'s `controlBelt` slot.
 *
 * Fail-closed throughout: no status yet means every indicator reads
 * `checking`, never a verdict the broker has not given.
 */
export default function ComposerControlBelt({
  status,
  models = [],
  roles = [],
  selectedModel = null,
  selectedRole = null,
  onModelChange,
  onRoleChange,
  disabled = false,
}: ComposerControlBeltProps) {
  return (
    <div
      role="group"
      aria-label="Composer controls"
      className="flex flex-wrap items-center gap-x-3 gap-y-2"
    >
      <BeltSelect
        label="Model"
        placeholder="Core default"
        title="Model selection is managed by Harness Core"
        options={models}
        value={selectedModel}
        onChange={onModelChange}
        disabled={disabled}
      />
      <BeltSelect
        label="Role"
        placeholder="Default"
        title="Role selection is not exposed by Core yet"
        options={roles}
        value={selectedRole}
        onChange={onRoleChange}
        disabled={disabled}
      />
      <div
        role="group"
        aria-label="Runtime context"
        className="ml-auto flex flex-wrap items-center gap-x-3 gap-y-1 font-figure-small-label"
      >
        {CONTEXT_COMPONENTS.map(([key, label]) => {
          const component = status?.[key]
          const state = component?.state ?? 'checking'
          return (
            <span
              key={key}
              data-state={state}
              title={component?.detail}
              className={
                'before:mr-1.5 before:inline-block before:size-1.5 before:rounded-full ' +
                'before:align-middle before:bg-current before:content-[""] ' +
                STATE_TONE[state]
              }
            >
              {label}: {state}
            </span>
          )
        })}
      </div>
    </div>
  )
}
