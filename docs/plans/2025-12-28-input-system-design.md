# Input System Redesign (Global-Consistent, Extensible)

Date: 2025-12-28

## Goals
- Make input behavior predictable and friendly for new users.
- Preserve Vim-familiar navigation (j/k/h/l, gg/G, etc.) across all tabs.
- Ensure mouse behavior matches keyboard semantics.
- Provide a clean, extensible input abstraction without user keymap config.

## Non-goals
- User-customizable keybinding files.
- Large refactors unrelated to input, or feature changes to existing tools.

## Interaction Contract
- **Modes**: Normal, Command, Prompt.
  - `:` enters Command mode.
  - `/` enters Search mode (same input bar, different parser).
  - `Esc` exits Command/Prompt/Search, returning to Normal.
- **Global consistency**: same keys mean the same action everywhere.
  - `j/k` or `Up/Down`: move selection within the focused panel.
  - `h/l` or `Left/Right`: move focus to adjacent panel.
  - `Tab/Shift-Tab`: cycle focus in a fixed order.
  - `Enter`: activate selected item in focused panel.
  - `1-5`: switch tabs globally.
  - `?` help, `s` settings, `q` quit, `y` copy, `Space` pause, `r` refresh.
- **Mouse**: click sets focus; click item selects; scroll affects panel under cursor.
- **Key repeat**: treat `Press` and `Repeat` as active input events.

## Architecture

### Input Router
Introduce a pure `InputRouter` that maps `KeyEvent`/`MouseEvent` to `UiAction`.

Inputs:
- `InputContext` (mode, current_tab, focus, selection state)
- `UiLayout` (panel rects, list inner rects, row maps)

Outputs (examples):
- `UiAction::FocusNext/Prev`
- `UiAction::MoveSelection(Up/Down)`
- `UiAction::Activate`
- `UiAction::SwitchTab(n)`
- `UiAction::EnterCommand` / `UiAction::EnterSearch` / `UiAction::ExitMode`
- `UiAction::MouseSelect(panel, index)`

Routing priority:
1) Modal overlays (help/settings/command/prompt) capture first
2) Global actions (quit/help/settings/command/search/pause/refresh/tab switch)
3) Focused-panel actions (move selection, activate, scroll)
4) Optional tab-specific actions (only if they do not violate global meanings)

### Single Source of Truth
- Remove `explorer_section` as a separate state.
- Sidebar selection derives from `active_section` only.
- All panels use the same selection/focus machinery.

### Command Bar State
- Replace scattered input buffers with a single `CommandBarState`:
  - `kind: Command | Search | Prompt`
  - `input: String`
  - `context: Option<String>`
- Rendering uses `kind` to decide prefix and hint text.

## Mouse/Hit Testing
- Create `ui::hit_test` to share layout data with input logic.
- Rendering produces a `RowMap` for sidebars with headings (Toolkit), so mouse
  clicks map to actual tools (not raw row indices).
- Mouse scroll maps to the panel under the cursor.

## Testing
- Unit tests for `InputRouter`:
  - Mode precedence (modal capture).
  - Global vs focused actions.
  - Focus cycling order.
  - `:` and `/` mode entry.
- Unit tests for row maps (Toolkit sidebar).
- A small integration-style test that feeds a sequence of key events to the
  router and asserts emitted `UiAction` results.

## Migration Plan
1) Add `UiAction` and `InputRouter` (no behavior changes yet).
2) Implement `UiLayout` + `RowMap` builders in UI rendering.
3) Replace current `handle_key`/`handle_mouse` with router output.
4) Remove `explorer_section` state and align sidebar selection to `active_section`.
5) Align Command/Search entry with `:` and `/` across UI hints.
6) Enable key repeat handling (`Press` and `Repeat`).

## Risks
- Partial migration can introduce duplicate handling; remove old paths fully.
- Ensure `UiLayout` is built from the same data as render to avoid drift.

## Success Criteria
- `:` always opens command mode; `/` always opens search.
- Mouse and keyboard actions match across tabs.
- Holding `j/k` scrolls properly.
- Explorer sidebar selection matches list content.
- Toolkit sidebar click selects the correct tool.
