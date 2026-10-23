# Stage 7b — bounded multiline root paragraph drafts

Successor [single-line root paragraph7a](testing-stage7a.md). Adds a separate native
multiline draft editor; the single-line grammar/panel/APIs remain unchanged.
Not free inline Preview typing, block conversion or a second document authority.

## Product contract

- **Edit multiline…** appears on eligible root paragraph provenance bars,136×24,
  same existing bar height/spacing. Single-line Edit paragraph, heading/table
  controls and menu prefix unchanged. Show block source remains (Fill shrinks on
  newly eligible multiline bars); not all previous source hit regions are identical.
- Only root Paragraphs of **2–128 nonempty unindented physical lines**, total raw
  payload **16KiB bytes**, final LF required. No leading whitespace on any line,
  blank lines, tab/CR/other control characters. Internal LF allowed. Payload
  excludes only the final LF; soft/hard breaks, trailing spaces, Unicode, markup,
  escaped pipe/entity and raw inline literal remain raw/editable, not normalized.
- Native **Enter inserts a draft newline**, never Apply/list continuation. Use the
  **Apply button** explicitly; no new Apply shortcut. Cancel/Escape discards only
  current draft. No Source or draft-session Undo while modal; after closing,
  each effective Apply is one Atomic PreviewParagraph edge in existing history.
- Draft is local `text_editor::Content`, not Document. Shared exact-paste helper
  protects native final LF/control scalars; draft serialization joins real native
  lines without inventing an LF.16KiB insertion/paste guard keeps last accepted
  bytes/selection on oversized input; invalid intermediate blank/indented/single
  line drafts remain editable. Moving/selecting/copying does not clear parser errors.
- Full target snapshot/block/payload must match **warm current root** metadata.
  Multiline and single-line APIs cannot authenticate one another's targets.
  Cold/stale/forged/nested/inner fenced ranges do not rebase or warm a cold model.
  Lexical scanners/affordances are not root/parser authentication.
- Effective Apply preflights **full candidate Source parse** before mutation.
  Same complete root Paragraph range and canonical multiline payload must remain;
  split paragraphs, heading/list/quote/fence/HTML block, blank lines or conversion
  to single-line are refused. No partial apply. Unchanged Apply closes without
  revision/history/redo/cache/caret/checkpoint changes.
- One payload replacement only: final LF, prefix/suffix/neighbor bytes untouched.
  Source caret is at applied **last-line payload end**, not draft cursor/glyph.
  Named native draft focus scope uses public Iced widget operations, not focus_next
  or runtime hooks; Source/query widgets are not touched. Parser refusal restores
  draft input focus; final Apply/Cancel restores existing Source/query context.
- Multiline, single-line and cell drafts mutually exclusive. Active modal blocks
  document/file/history/mode/new-draft commands; Source mode/pending confirmation/
  Open reject Open/Apply. Inflight Save/Export snapshots/replies stay immutable;
  draft survives async acknowledgments. Search refresh on effective Apply only,
  logical Find/Go-to preserved, physical query/Go-to focus still manual.
- History200entries/8MiB, Source typing barriers, save identity/export pipeline
  unchanged. Full parse/reconstruction/history O(n), not speedup/benchmark.

## Implementation and tests

`preview_paragraph_multiline` keeps7a's Target shape but separate bounded scanner/
plan. `Block::multiline_paragraph_payload` is assigned only to root Paragraphs;
new adapter/Message/API/modal/editor are separate so7a's unsupported multiline
oracles remain exact. `source_focus::task_in` reuses scoped public operation;
existing Source task/tests retained. Source `buffer_text`/`paste_exact` helpers
are reexported internally unchanged; no new serialization convention/buffer authority.

**829 Cargo passed** =801 old +**28 Rust**:5 planner +7 Document +3 adapter/model
+8 controller +4 native-draft UI +1 focus-scope. **282 strict core** subset Cargo,
not282 additional unique tests. Python **53 passed** =51 old +2 independent
literal/action/oracle/exclusive-dispatch probes; private DISPLAY guard15suites.

- Planner raw soft/hard breaks/trailing spaces/Unicode/markup/entity/escaped pipes/
  prefix/suffix/caret, byte/line caps/overflow, blank/indent/control/single/EOF,
  forged snapshot/ranges/UTF8, no-op/lexical inner-fence/structural-positive examples.
- Document raw bytes/full candidate parser/one-step Undo/Redo/caret/cache, cold/
  nine forged/stale/replace/cross-API auth, no-op/structure/cap refusal preserving
  redo/revision/caret/cache/checkpoint, nested/literal/blank/indent/EOF, fresh target/
  neighboring roots, delayed save/branch/Source barrier, exact16KiB/128line capacity.
- Adapter/model exact metadata/Message targets, root-only/unsupported/budget,
  single-line adapter unchanged/inert for multiline and vice versa.
- Controller native Enter is staging/Cancel, Preview/Split atomic Apply/caret,
  no-op close/redo, invalid/structural/oversized retention/correction, Source/prompt/
  Open/cell/single/no-draft guards, active modal blocks commands, immutable Save/
  Export and replies, logical Find/Go-to/search, stale draft/native Source barrier.
- UI raw paste retains spaces/final LF/invalid final tab, correction, exact byte cap/
  large-paste refusal preserving selection, Enter/native vs global shortcut bindings,
  lexical Apply/error gating. Scope test proves idempotence/no Source/query crosstalk.

## Private GUI suite15

`--suite paragraph-multiline` gets private app/Xvfb/XDG/env/clipboard,180s runner
alarm/5min step of its own; job45min unchanged. All **25 old exercise bodies**
(12 regression phases +13 independent suites) AST-identical against `9ad722d`.
All14 old gates **116+9+9+9+14+16+11+11+11+11+11+11+16+15** retained unchanged;
no retry/runtime hooks/old coordinate/oracle/deadline/refocus changes.

**16 checks passed** =15 exact Copy (11 Source +4 draft) +natural clean Close0:

1. Exact welcome/fixture/raw hard-break payload and scoped native editor focus.
2. Native typing/Enter produces `discard\npending` in draft, still modal; Cancel
   preserves Source/no history. Native Enter is not Apply or Source continuation.
3. Unicode3-line clipboard staging; explicit Apply preserves final LF/neighbors/
   suffix/tab, one Preview Undo/Redo restores exact raw source.
4. Split unchanged Apply closes no edge. `first\n# heading` Apply refused by parser;
   exact draft Copy without refocus proves input focus restored. Cancel preserves
   Source, not partial first-line commit.
5. Split final Apply then native X without Source refocus yields `final\nnextX`
   before suffix; caret at last applied line end. Separate typing/Apply/paste Undo
   reaches welcome clean, natural client Close0.

Nine clicks only product Edit multiline/Apply controls, no Source/draft refocus
workaround. Screenshots `paragraph-multiline/multiline-preview.png`,
`multiline-invalid.png`, `multiline-split.png` are not pixel/glyph oracles. Observed
**286 total GUI** =258 clipboard (236 Source +22 draft),12 confirmation titles,
16 natural exits.

```sh
./scripts/test-gui-x11.sh --suite paragraph-multiline --artifacts "$(mktemp -d)"
# Run all14 old suites too, never substitute.
```

## Execution status

Local format/harness format, **282 strict core**, **53 Python**, diff check pass.
[CI 39881731180](https://github.com/Indrawan007/Hylite/actions/runs/39881731180),
`5d9cca1e6f4968b26fde59106aa6fb1c931d2543`, **success**, watch exit0;
job `120180464018`,23 October2026 **01:03:04–01:11:19 UTC** /
**09:03:04–09:11:19 Makassar**. Native format/strict Clippy/debug build/829Cargo
and all15GUI suites pass. Annotations independently report116/9/9/9/14/16/11/11/
11/11/11/11/16/15/16GUI,829Cargo. Core subset/Python counts come from the local
same-revision runs; their CI steps also pass (not extra native unique Rust tests).

Artifact `12470923520` (`gui-x11-smoke`), **2,913,840 bytes**, unexpired when checked;
SHA256 `a94ab4c472bc597b58e6a22f360133a666b7e600e1cdbf86ff0cfc5bca6e75d3`,
expires2026-10-30T01:11:15Z. Screenshots not visually reviewed. Full archived log
fetch was blocked at Actions results host; native results above are successful
run/job/steps and check-run annotations, not invented full-log evidence.
Previous docs7a run39880919017 cancelled superseded feature7b;7a feature
39879922775 remains success. No main merge.

## Manual / limits

- [ ]128lines/16KiB/trailing spaces/other structural failures UI; selection/caret/
  keyboard traversal/accessibility/narrow window/scroll/glyph visuals; physical
  query/Go-to focus and other Source-bar hit regions. Unit guards aren't pixel proof.
- [ ]Arch/Hyprland/Wayland/portal/IPC/GPU/IME/RTL/HiDPI. X11 tiny-skia is not target
  compositor verification. Artifact screenshots not visually reviewed.

No EOF/indented/blank-separated paragraph editing, block conversion/creation/delete/
reorder, free Preview inline typing/rich selection formatting/multi-block selection,
draft-session Undo, glyph↔caret mapping, sync scroll/incremental parse/virtualization/
speed-RAM benchmark. Source LF/gutter pending and historical startup6b exit101 still
undiagnosed; this does not claim to fix startup/pixel layout.
