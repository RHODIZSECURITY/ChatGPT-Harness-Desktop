import { useId } from 'react'
import { useTheme } from '../theme/ThemeProvider'
import { THEME_PREFERENCES, type ThemePreference } from '../theme/theme'

/**
 * Labels are keyed off the preference tuple rather than listed beside it, so a
 * fourth preference cannot be added without the compiler asking for its word.
 */
const LABELS: Record<ThemePreference, string> = {
  dark: 'Dark',
  light: 'Light',
  system: 'Auto',
}

/**
 * The theme picker.
 *
 * Native radios, visually hidden behind their labels, rather than three
 * buttons carrying `role="radio"`. A hand-written radiogroup owes the user
 * roving tabindex and arrow-key movement between options, and a control that
 * looks grouped but tabs through three separate stops is the kind of detail
 * that is invisible until it is wrong. The browser already does it.
 *
 * Words, not glyphs. The icon set has a sun and a moon but nothing that means
 * "follow the operating system" — the usual half-filled circle is not in it —
 * and a circle or a lightbulb pressed into that role would teach the wrong
 * thing. Three words are unambiguous, and they stay legible at any width.
 */
export default function ThemeControl() {
  const { preference, setPreference } = useTheme()
  const name = useId()

  return (
    <fieldset
      className="flex gap-0.5 rounded-08 border border-border-01 bg-background-tint-01 p-0.5"
    >
      <legend className="sr-only">Theme</legend>
      {THEME_PREFERENCES.map((option) => (
        <div key={option} className="flex-1">
          <input
            type="radio"
            id={`${name}-${option}`}
            name={name}
            value={option}
            checked={preference === option}
            onChange={() => setPreference(option)}
            className="peer sr-only"
          />
          <label
            htmlFor={`${name}-${option}`}
            className={
              'block cursor-pointer rounded-04 px-2 py-1 text-center font-secondary-action ' +
              'text-text-03 transition-colors hover:text-text-04 ' +
              'peer-checked:bg-background-tint-03 peer-checked:text-text-05 ' +
              // The input is hidden, so its focus ring has to be borrowed. Without
              // this the control is keyboard-operable and gives no sign of it.
              'peer-focus-visible:outline peer-focus-visible:outline-2 ' +
              'peer-focus-visible:outline-offset-1 peer-focus-visible:outline-border-03'
            }
          >
            {LABELS[option]}
          </label>
        </div>
      ))}
    </fieldset>
  )
}
