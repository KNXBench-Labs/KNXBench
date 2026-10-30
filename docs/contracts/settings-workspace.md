# Settings workspace

## Scope

Settings is a normal center-workspace page, not a modal or an additional
window. The sidebar and command-palette Settings actions use the same page,
including when no project is open. This source change is newer than Alpha 18;
the published Alpha 18 settings window is unchanged.

## Presentation and navigation

- All existing app settings appear on one continuous page, grouped into sections.
  A compact jump navigation above the sections leads to their headings; it does
  not hide other categories or create separate category pages.
- Left app navigation stays usable. Selecting Settings again keeps the same
  settings page active, rather than toggling it closed or stacking another page.
- Project Properties is not shown alongside app settings. Leaving Settings
  restores its existing visibility preference; opening Settings must not change
  that preference or persist a new pane width.
- The page is reachable without an open project. It has normal page semantics,
  no modal backdrop, modal focus trap, resize handle or Escape-to-dismiss rule.
- Keyboard entry gives the page a clear focus destination. Jump navigation
  reaches and focuses its target headings without trapping focus or making
  scrolling depend on animation.
- Sections and their controls stay reachable in a narrow viewport and at the
  supported app zoom levels. Wide layouts may use available center-pane space;
  the settings page is not positioned over the app's other panes.

## State and mutation boundaries

Switching to Settings and back preserves the current project selection and
unsaved project-note/device-instructions drafts. View changes must not reset
active operation owners or silently save, discard or otherwise mutate a project.
A retained comparison or other workspace dialog must release its foreground
isolation while its owner is hidden: it must not make Settings or navigation
inert. Returning to that workspace resumes its existing modal order, draft,
last dialog focus and background protection, rather than reopening or cancelling
the operation. Ordinary dialogs, including settings-child confirmations, keep
their modal focus and isolation behavior.

The existing setting-specific immediate-apply, acknowledged-persistence,
validation and failure-feedback paths remain authoritative. This change adds no
global Save action, no settings fields, no new settings scope or migration, and
no automatic bus operation. Existing explicit confirmations and safety refusals
remain in force. Changing UI language updates the page without reopening it.

## Verification boundary

Regression coverage must exercise the mounted app's real entry points, not only
a standalone settings component: projectless and loaded-project navigation,
prior-open/prior-closed Properties, preserved selection and drafts, repeated
entry, settings changes and failure feedback, EN/DE, wide/narrow layout and
keyboard jump reachability. Browser API fixtures must intercept discovery as
well as ordinary settings requests. Browser evidence is not native desktop or
hardware acceptance; release screenshots remain pinned to their released source.
