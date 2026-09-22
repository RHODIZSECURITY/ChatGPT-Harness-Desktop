import { useId } from 'react'
import { Button, InputSingleSelect, Text } from '@opal/components'
import { SvgMoon, SvgSun } from '@opal/icons'
import { useSidebarFolded } from '@opal/layouts'
import type { IconFunctionComponent } from '@opal/types'
import { useTheme } from '../theme/ThemeProvider'
import { isThemePreference, THEME_PREFERENCES, type ThemePreference } from '../theme/theme'
import SvgThemeAuto from './SvgThemeAuto'

const LABELS: Record<ThemePreference, string> = {
  dark: 'Dark',
  light: 'Light',
  system: 'Auto',
}

/// "Auto" is the only one of the three that does not describe what you will
/// see, so every option carries a line rather than singling that one out with
/// an explanation the others lack.
const DESCRIPTIONS: Record<ThemePreference, string> = {
  dark: 'Always dark',
  light: 'Always light',
  system: 'Follow the system',
}

/// The icon shows the *preference*, not the resolved theme. Resolved would be
/// the wrong thing to draw: the window itself is already the largest possible
/// statement of which side is in force, and an icon that repeats it makes Auto
/// indistinguishable from whichever side it happens to have landed on.
const ICONS: Record<ThemePreference, IconFunctionComponent> = {
  dark: SvgMoon,
  light: SvgSun,
  system: SvgThemeAuto,
}

function nextPreference(current: ThemePreference): ThemePreference {
  const index = THEME_PREFERENCES.indexOf(current)
  return THEME_PREFERENCES[(index + 1) % THEME_PREFERENCES.length]
}

/**
 * The expanded control: a labelled select.
 *
 * Opal's select, not the platform's: a native `<select>` draws its popup with
 * operating-system chrome, which is the one surface in this window that no
 * token can reach and that therefore looks bolted on in both themes.
 *
 * Name and value are kept apart, the way a native `<select>` with a `<label>`
 * keeps them: the visible "Theme" caption is the name, referenced through
 * `aria-labelledby`, and the current preference is the trigger's own content.
 * An `aria-label` would *replace* that content, so the control would read as
 * "Theme" whether it said Dark or Light — a label that hides the state it
 * labels.
 */
function ThemeSelect() {
  const { preference, setPreference } = useTheme()
  const id = useId()
  const labelId = `${id}-label`

  return (
    <div className="flex flex-col gap-1.5">
      <Text id={labelId} font="figure-small-label" color="text-03">
        Theme
      </Text>
      <InputSingleSelect
        value={preference}
        onValueChange={(next) => {
          // The select is typed as a plain string. Narrowing here keeps the
          // preference model the only place that says what a theme can be.
          if (isThemePreference(next)) setPreference(next)
        }}
      >
        <InputSingleSelect.Trigger aria-labelledby={labelId} />
        <InputSingleSelect.Content>
          {THEME_PREFERENCES.map((option) => (
            <InputSingleSelect.Item
              key={option}
              value={option}
              description={DESCRIPTIONS[option]}
            >
              {LABELS[option]}
            </InputSingleSelect.Item>
          ))}
        </InputSingleSelect.Content>
      </InputSingleSelect>
    </div>
  )
}

/**
 * The folded control: one icon that advances through the same three values.
 *
 * A 3.25rem rail cannot hold a select — the trigger is a bordered input box
 * with a chevron, and Opal's is `WithoutStyles`, so it cannot be talked down
 * to icon size. Hiding the control instead, which is what this file used to
 * do, costs the user the theme entirely for as long as the sidebar is folded.
 * Cycling is what a rail can afford, and it is reversible in two clicks.
 *
 * Name and value stay split here too, but in the only way a control with no
 * text can manage it: the label states the subject, the current value and the
 * next one, so pressing it is never a guess. Same `setPreference` as the
 * select, so there is one writer and no state to keep in step.
 */
function ThemeCycle() {
  const { preference, setPreference } = useTheme()
  const next = nextPreference(preference)
  const label = `Theme: ${LABELS[preference]} — switch to ${LABELS[next]}`

  return (
    <div className="flex justify-center">
      <Button
        icon={ICONS[preference]}
        prominence="tertiary"
        size="md"
        aria-label={label}
        tooltip={label}
        // The rail is against the window's left edge; anywhere else and the
        // tooltip opens over the sidebar it belongs to.
        tooltipSide="right"
        onClick={() => setPreference(next)}
      />
    </div>
  )
}

/**
 * The window's theme, in whichever form the sidebar currently has room for.
 *
 * Opal hides the sidebar *body* when folded but leaves the footer visible, so
 * what a footer does at rail width is this component's problem and not the
 * layout's. Two components rather than one branch: `useTheme` is called on
 * both paths, and a hook behind a condition is a hook that changes order.
 */
export default function ThemeControl() {
  return useSidebarFolded() ? <ThemeCycle /> : <ThemeSelect />
}
