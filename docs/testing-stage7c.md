# Stage 7c — multiline draft-local Undo/Redo

Successor to [7b multiline root drafts](testing-stage7b.md). Only the multiline
modal gains session history; single-line7a, cell panels and Document history APIs
are unchanged. [7d navigation](testing-stage7d.md) additionally starts a fresh local
history on successful unchanged-only Previous/Next; old-session redo is discarded.
This supersedes7b's **no draft-session Undo** limit, not its parser,
authentication, eligibility or Source-authority rules.

## Contract

- Focused native draft editor: **Ctrl+Z** Undo, **Ctrl+Y / Ctrl+Shift+Z** Redo.
  Exact Ctrl modifiers only (no AltGr/logo). Native custom bindings emit a separate
  `MultilineDraftHistory` message and capture even empty stacks, so keys cannot
  leak into Source/Preview history. Unfocused editor history shortcuts are inert.
  Direct Source `Undo`/`Redo`/`PreviewHistory` messages still blocked by modal.
- **Undo draft / Redo draft** buttons appended after existing Apply/Cancel in the
  same row, disabled when unavailable. Existing Apply/Cancel position, panel/root
  bar geometry and all old controls unchanged. Effective enabled button action
  restores named draft editor focus; no focus_next/runtime hook. Disabled-button
  clicks can blur native input and are not claimed to restore focus.
- **One effective native edit = one local step**, including paste/selection
  replacement, deletion and Enter. No typing coalescing, time thresholds or shared
  Source transaction state. Selection/navigation/copy/no-op/native deletion at
  start and oversized input refusal neither create a step nor discard redo.
  New effective edit after Undo discards only draft redo.
- Snapshots retain **exact raw bytes**, including lexically invalid intermediate
  drafts: blank/indented/single line, trailing LF/tab/control, hard break spaces,
  Unicode. Native reconstruction uses Source `content_at` helper reexported
  internally with body unchanged; exact bytes/logical caret verified **before**
  consuming history or replacing native content. Failed reconstruction leaves
  accepted draft and both stacks untouched.
- Caret is UTF-8 logical byte offset. Restore **collapses selection** at captured
  caret; does not restore anchor/direction/native scroll/visual column. Undo uses
  pre-edit caret; Redo stores current caret when Undo is performed. No glyph/
  grapheme/RTL selection mapping promise.
- Restored draft revalidates lexical Apply affordance. Invalid snapshots remain
  editable. Effective restore clears old parser errors, but **Apply still performs
  full Document target authentication and complete-root structural preflight**.
  Restoring `one\n# heading` cannot bypass rejection. Empty history preserves errors.
- Session history max **100 retained records across both stacks**, **1MiB retained
  snapshot text**, each raw record≤16KiB. Evict oldest Undo then farthest Redo until
  both budgets fit, including transfers between differently sized snapshots. No
  cap popup; budget bounds retained raw text, not total process/allocation memory.
- Open creates empty history. Apply/no-op close/Cancel/Escape drop entire current
  session. Reopen cannot Undo/Redo a previous draft. No persistent or shared cache,
  no secure-memory-erasure claim.
- Local history never changes Source bytes, snapshot/checkpoint/dirty state,
  revision/cache/search/caret or Source redo. Source still changes only through
  effective Apply as **one PreviewParagraph atomic edge**, irrespective of local
  edit count. Cancel/no-op no Source history. Inflight Save/Export snapshot/replies,
  stale target refusal, modal/file guards and logical Find/Go-to context retained.

## Implementation / regression coverage

`multiline_draft_history` is pure Rust bounded snapshot storage, no Document/Iced/
file/operation identity. Draft owns it and stores native byte caret, preflights
`content_at` before transfer; controller routes only contextual message to draft
and restores scoped native focus. `session_key_binding` wraps existing7b binding;
base app/Source/Preview shortcut functions unchanged. Only visibility/export of
Source `content_at` changes; body unchanged, no new text serialization convention.

**852 Cargo passed** =829 previous +**23 Rust** (**12 pure history +6 native UI
+5 controller**). Local **294 strict core** subset Cargo, **55 Python**. Native
Cargo/build/Clippy/all16GUI suites pass; actual evidence below.

- Pure: empty/inert, valid/invalid raw Unicode/hard breaks/final LF/control/empty,
  caret/no-op redo, local branching, malformed UTF8 caret/oversized record rejection,
 100-record eviction,1MiB cap, differently sized transfer rebudgeting, byte caret/
  final empty line, independent sessions, non-consuming target peek and1000-edit
  reference stack workflow.
- UI: raw invalid Undo/Redo and lexical Apply, byte caret/selection collapse/final
  LF, navigation/no-op/large paste do not discard redo, native Enter one step,
  branching, exact focused key/modifier context and unfocused/AltGr/Source command
  rejection, parser error retained on empty history/revalidated after effective
  restore. Existing4UI tests retained (base binding remains unchanged).
- Controller: local-only Source identity/cache/caret/dirty invariants, single atomic
  Apply in Preview/Split regardless of local edits/reopen empty, no-op close leaves
  Source redo, structural refusal after local redo/stale targets/no-draft/single
  modal isolation, immutable Save token/checkpoint +logical search refresh only at
  Apply. All old controller Document-Undo modal guards unchanged.

## Independent private GUI suite16

`--suite paragraph-draft-history` private app/Xvfb/XDG/env/clipboard;180s alarm,
5min independent step,45min job unchanged. All **26 old exercise bodies** AST-
identical against `91386ee`; all15existing gates retained. No old byte/coordinate/
oracle/refocus/deadline changes, no retry/runtime hooks.

**24 checks passed** =23 exact Copy (8 Source +15 draft) +natural Close0:

1. Clean welcome/fixture, empty draft Ctrl+Z captured; Unicode3-line paste one
   local step, Ctrl+Z→initial, Ctrl+Y and Ctrl+Shift+Z→entire paste.
2. Blank/final-tab invalid paste exact Copy, Undo valid/Redo invalid with no byte
   normalization; local branch clears redo, empty Ctrl+Y keeps current draft.
3. Native **Undo draft** click restores prior exact payload and input focus,
   Copy without refocus. Cancel discards local stacks, exact Source fixture remains.
4. Split reopen has empty redo. Final raw Unicode/trailing spaces staged, local
   Undo/Redo exact; explicit Apply then native X without Source refocus at payload
   end; Source typing/Apply Undo separately restore final/original Source.
5. Preview Redo restores only committed paragraph, not local snapshot; two Source
   Undo return welcome clean, natural client Close0. No hidden draft edges.

Four clicks only product Open/Undo draft/Apply. Screenshots
`paragraph-draft-history/draft-history-preview.png`, `draft-history-source.png`
are not pixel/glyph oracles. Observed aggregate **310 GUI** =281 clipboard
(244 Source +37 draft),12 confirmation titles,17 natural exits.

```sh
./scripts/test-gui-x11.sh --suite paragraph-draft-history --artifacts "$(mktemp -d)"
# Keep all15previous suites too, never substitute.
```

## Execution status

Local format/harness format,**294strict core**,**55Python**,diff check pass.
[CI 39884654030](https://github.com/Indrawan007/Hylite/actions/runs/39884654030),
verified `125365525c5d6a51e4e952d549940c516b144326`, **success**, watcher exit0;
job `120189875625`,23 October2026 **01:33:21–01:42:33 UTC** /
**09:33:21–09:42:33 Makassar**. Native format/strict Clippy/debug build/**852Cargo**
and all16GUI suites pass. Check annotations report116/9/9/9/14/16/11/11/11/11/11/
11/16/15/16/24 =**310GUI** and852Cargo. Core/Python counts are local same-revision
runs; corresponding CI steps also pass, not extra unique native Rust tests.

Artifact `12472357915`, `gui-x11-smoke`, **3,003,957 bytes**, unexpired when checked;
API SHA256 `4ca8c3dddc53352fa748726efcd786766238845e088ac2590de2b102aaf3592a`,
expires2026-10-30T01:42:29Z. Evidence from run/job/steps/check annotations and artifact
metadata, not full archived-log or visual screenshot inspection.

Initial [39883632691](https://github.com/Indrawan007/Hylite/actions/runs/39883632691),
feature `52e77a0819efb45af5392442b6bfe559761e9f8a`, failed **new probe only** after
21successful Copies: expected Preview Redo inspection but probe mistakenly cycled
Split→Source→Preview around Redo, leaving Copy in Preview (sentinel/no clipboard
ack).852Rust and all15oldGUI gates passed there. Test-only1253655 enters Preview
via Split→Source→Preview **before** Ctrl+Y, then Split for Copy. Same expected raw
bytes/coordinates/deadlines/no-refocus remain; Python now asserts mode sequence.
No application-code correction, rerun/retry hook, widened tolerance or old-gate
weakening. Complete fresh CI39884654030 passes all16suites. Previous7b docs
39882726245 success. No main merge.

## Manual / limits

- [ ]History UI at100records/1MiB, other native edit/selection/clipboard variants,
  narrow window/button traversal/disabled click blur/accessibility/physical query-
  Go-to focus, scroll/layout/glyph visuals. Unit caps are not pixel/UI capacity proof.
- [ ]Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU. X11 tiny-skia is not target
  compositor proof; screenshots not visually reviewed.

No local history for single-line/cell drafts, typing coalescing, restored selection
anchor/scroll, persistent history, block conversion/free inline Preview editing/
glyph mapping/multi-block selection/sync scroll/incremental parsing/performance
claims.7b LF-root2–128nonempty/unindented lines/16KiB remains; EOF/blank-separated/
indented paragraph unsupported. Full draft snapshots/reconstruction O(n), not speedup.
