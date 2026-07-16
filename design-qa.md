# Design QA — list-page hierarchy and status filter cleanup

## Evidence

- Source visual truth: `/var/folders/qx/5mn_ty6s1tjffl52859575s40000gn/T/TemporaryItems/NSIRD_screencaptureui_OKu4E1/스크린샷 2026-07-15 오전 10.39.27.png`
- Closed list implementation: `/Users/jinsewoon/Desktop/projects/work-harvest/design-qa-artifacts/list-cleaned.jpg`
- Open status-filter implementation: `/Users/jinsewoon/Desktop/projects/work-harvest/design-qa-artifacts/status-menu-open.jpg`
- Full-view comparison: `/Users/jinsewoon/Desktop/projects/work-harvest/design-qa-artifacts/list-cleanup-full-comparison.jpg`
- Focused comparison: `/Users/jinsewoon/Desktop/projects/work-harvest/design-qa-artifacts/list-cleanup-focused-comparison.jpg`
- Native app: `/Users/jinsewoon/Desktop/projects/work-harvest/target/debug/bundle/macos/Work Harvest.app`
- Viewport: 1398 × 768, light theme, 2026/07/14 selected.

## Findings

- No actionable P0/P1/P2 issue remains.
- Information hierarchy: the redundant `업무 기록` label, top-bar total, and `선택한 날짜` label are removed. The selected date and `업무 목록` are the only two headings required to orient the screen.
- Typography and spacing: `1개` now aligns to the `업무 목록` title baseline. Search and status controls form one consistent 34px control row.
- Colors and selection: the list item has no default selected background. Hover remains the only list-row emphasis because clicking opens a separate detail page.
- Controls and icons: the OS-native select is replaced with an app-owned popover listbox that uses the same border, radius, shadow, icon stroke, and focus treatment as the rest of the interface.
- Accessibility: the filter trigger exposes `aria-haspopup="listbox"` and expanded state; options expose `role="option"` and selection state. All interactions remain keyboard-focusable buttons.
- Image quality and copy: no imagery is required. Existing Lucide icons remain appropriate and all user-facing copy is concise and coherent.

## Comparison history

### Pass 1 — passed

- Removed the three redundant labels/counts identified in the source screenshot.
- Realigned the list count, removed persistent default row selection, and replaced the native select.
- Verified the closed list and opened status-filter states in the native debug app. No P0/P1/P2 mismatch remains.

## Primary interactions tested

- Loaded the list page and confirmed the redundant header metadata is absent.
- Confirmed the top row is not visually selected by default.
- Opened the status filter and confirmed its custom menu, selected mark, and options render inside the app surface.
- Confirmed native debug build showed no crash or visible runtime error.

## Implementation checklist

- [x] Remove `업무 기록` top-bar label
- [x] Remove top-bar total count
- [x] Remove `선택한 날짜` list label
- [x] Align list count with title
- [x] Remove default list-row background selection
- [x] Replace native select with custom listbox

final result: passed
