# TerminalView

**Inventory:** W44 · adapter boundary · P6 · extension component\
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and boundary

TerminalView is a generic prepared-cell terminal presentation component. It
borrows a caller-provided terminal-cell snapshot, supported mode/selection
metadata and stable history coordinates. It preserves cell styles, wide-cell
continuations, cursor state and selection while providing typed interaction
requests.

This is a new reusable adapter boundary extracted from the Jackin preview's
app-local terminal pane. It has no standalone legacy widget family. Baseline
application regions and new adapter behavior are verified in separate lanes.

Termrock does not become:

- a terminal emulator;
- a PTY runtime;
- a shell;
- a daemon;
- an agent-session manager;
- a terminal escape parser.

Those concerns remain external. A future application adapter supplies terminal
cells and retains raw input/session/process state.

## Oracle and provenance

- [Frozen capsule composition](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/bin/jackin_preview/screens/capsule.rs)
- [Frozen PTY simulator](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/bin/jackin_preview/sim/pty.rs)
- Snapshot discovery: frozen Jackin capsule views.
- [TerminalView capture plan](../reference/capture-plans/terminal-view.json)
- [ScrollRegion contract](./scroll-region.md), [runtime](../foundations/runtime.md),
  [text](../foundations/text.md) and [conformance](../foundations/conformance.md)

The capsule and PTY files are preserved integration evidence. They do not
make PTY/session code part of the Termrock library.

## Target public API

    TerminalView::new(
        id: Id,
        source: &'a dyn TerminalSource,
    ) -> TerminalView<'a>

    update(
        &self,
        cx: &mut Cx<'_>,
        state: &mut TerminalViewState,
    ) -> Response<TerminalAction>
    draw(
        &self,
        ui: &mut Ui<'_>,
        area: Rect,
        state: &TerminalViewState,
    ) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builders:

    interaction(TerminalInteraction)
    selection(Option<TerminalSelection>)
    dimmed(bool)
    patch(StylePatch)   // chrome only; child cell colors remain intact

TerminalViewState contains a view offset and selection anchor in stable
line/cell coordinates. The committed selection is caller-controlled. Safe
read-only observations expose source-keyed selection/caret and reading offset;
the component does not expose a global Buffer or source collection.

Typed actions:

    TerminalAction::Forward { token: InputToken }
    TerminalAction::CopyRequested(TerminalSelection)
    TerminalAction::OpenLink { key: LinkKey }

Forwarding returns the host-retained input token. The view cannot reconstruct
bytes that the host already discarded.

## Ownership and phases

| Concern | Owner |
| --- | --- |
| Prepared cells, styles, continuation metadata and source history | Caller-owned TerminalSource |
| View offset and selection anchor | TerminalViewState |
| Committed selection policy and clipboard/export | Caller/application |
| Focus, hit testing and pointer capture | Runtime and layer owner |
| Terminal mode authority, parser, PTY and process lifetime | External adapter |
| Resize I/O | Host adapter after draw reports facts |

update handles scrolling, selection gestures, link/copy requests and input
forwarding after the outer modal/prefix layer gives permission. draw blits the
borrowed cells and reports inner viewport/cursor facts without performing I/O.
measure and draw agree on viewport geometry and wide-cell handling.

Outer modal or prefix ownership wins before forwarding. Normal terminal mode
emits Forward; selection mode emits source-coordinate CopyRequested, subject
to stale-history rejection. A link action carries a stable LinkKey and leaves
safety policy to the host.

## Visual contract

Parts are container, terminal-cell, cursor, selection and link. Original cell
foreground, background, modifiers, wide-cell continuation and combining
symbols are preserved. The view does not recolor child terminal output to the
Termrock theme. A dimmed presentation is applied only when the baseline
overlay policy asks for it.

Cursor visibility and position, selection highlight, clipping, scrollbars,
edge fades and focus/inactive pane treatment are exact proof fields. Unicode
width and continuation cells come from the prepared source; Termrock must not
reinterpret or split them. Theme patches apply to view chrome only.

## State matrix

| Axis | Required states |
| --- | --- |
| Geometry | normal, zero/tiny area, nonzero origin, exact fit, 1-cell short, long/combining Unicode, narrow then wide |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover |
| Activation | No generic whole-surface activation; scroll/selection gestures and typed requests apply |
| Scroll | no overflow, start/middle/end, wheel boundary, thumb drag, resize while scrolled, edge fade |
| Output | source replaced, source appended, selected cells, source copy, clipped cursor |
| Retention | history evicted, selection evicted, retained reading anchor |
| Modal/input | modal intercept, prefix intercept, normal forwarding |
| Resize/I/O | reports viewport/cursor facts; never resizes a PTY as a paint side effect |
| Color/protocol | original cell colors, continuation metadata and explicitly classified new attributes |

## Required parity and extension cases

| Case | Authority and observation |
| --- | --- |
| W44-01 | Extension/oracle subregion: borrowed cells with wide cells, combining symbols and cursor visibility |
| W44-02 | Extension/oracle subregion: focused, inactive and dimmed terminal regions |
| W44-03 | Extension/oracle subregion: modal intercepts; normal mode emits forwarding token |
| W44-04 | Extension/oracle subregion: source-coordinate copy and stale-history rejection |
| W44-05 | Extension/oracle subregion: viewport resize reports facts and performs no I/O |
| W44-06 | Extension disposition: new protocol attributes are explicitly outside baseline proof |

Record exact dimensions, symbols, continuation metadata, foreground/background,
modifiers supported by the comparator, cursor position/visibility, focus and
capture owners, selection coordinates, offset, source revision, action count,
action target and forwarded token identity. No candidate code may create or
approve its own expected artifacts.

## Foundations and negative tests

Depends on [identity](../foundations/identity.md), [input/actions](../foundations/input-actions.md),
[runtime](../foundations/runtime.md), [layers](../foundations/layers.md),
[layout](../foundations/layout.md), [text](../foundations/text.md),
[authoring](../foundations/author.md), [session](../foundations/session.md) and
[conformance](../foundations/conformance.md).

Required negative tests:

- TerminalView cannot parse escape sequences or spawn/own a PTY;
- draw cannot perform resize, clipboard, link, daemon or process I/O;
- modal/prefix ownership cannot be bypassed by forwarding;
- a stale history coordinate cannot copy a replacement line;
- wide-cell continuation and combining symbols cannot be lost or recolored;
- selection/caret state cannot escape stable source coordinates;
- a candidate cannot generate or bless its own expected snapshot;
- source replacement cannot silently retarget a selection or cursor.
