# Stage 7a — single-line root paragraph raw drafts

Successor [Apply & Previous6p](testing-stage6p.md). First bounded paragraph editing
outside task/heading/table controls, not general inline WYSIWYG or glyph mapping.

## Kontrak produk

- **Edit paragraph…** pada existing root paragraph provenance bar membuka modal
  draft raw Markdown. Button136×24 append setelah Show block source; bar height,
  content spacing, heading/table controls dan menu lama tidak berubah. Paragraph
  Show block source tetap ada (Fill width berkurang untuk button baru).
- Hanya **single physical-line, unindented, nonempty root paragraph dengan LF**.
  Payload adalah raw paragraph bytes tanpa final LF, maksimal16KiB. Tidak ada
  leading whitespace/control characters (termasuk tab/CR/LF); trailing spaces,
  Unicode/markup/entity/escaped pipes/raw inline literal tidak dinormalisasi.
  Trailing spaces adalah bagian payload editable, bukan whitespace luar sel.
  Multiline/indent/EOF tanpa LF/nested/list/quote/heading/code/HTML block tidak
  memperoleh action. Ini subset konservatif, bukan semua paragraf Markdown.
- **Enter/Apply** mengganti satu payload, bukan serialisasi AST. Final LF, prefix,
  suffix, neighboring blocks, bytes raw di luar payload tetap. Candidate **full
  Source parse** harus masih memiliki **complete root Paragraph range tepat**
  dengan canonical payload baru. Heading/list/quote/rule/fence/HTML block atau
  absorbed/split paragraph ditolak sebelum mutation. Valid lexical text saja
  tidak cukup; raw inline markup/link/HTML tetap inert sesuai renderer lama.
- One effective Apply = **satu Atomic PreviewParagraph edge**. Preview Undo/Redo
  dapat memulihkan edge ini tetapi tidak melintasi Source barrier. Source caret
  di **akhir payload yang diterapkan**, bukan glyph posisi yang diklik.
- **Cancel/Escape** membuang draft tanpa Source/revision/history/dirty/cache/caret
  mutation. Unchanged Apply menutup tanpa normalisasi/history/redo clearing.
  Invalid/stale/preflight rejection mempertahankan draft; input oversized menjaga
  last accepted value dan menampilkan error. Perbaiki atau
  Cancel. Tidak ada implicit Apply ketika mengganti mode/file/history/navigation.
- Target wajib current full snapshot + exact block/payload ranges, warm current
  **root** projection membership dan paragraph metadata. Cold/forged/stale/inner
  literal/nested range tidak di-rebase atau diautentikasi hanya lewat scanner.
- Source tetap authority. Draft mutually exclusive dengan cell draft; active
  paragraph draft memblokir document/file/history/mode/new-draft commands. Source
  mode/pending confirmation/Open menolak Open/Apply. Inflight Save/Export tetap
  immutable snapshot awal; replies tidak membuang draft dan checkpoint mengikuti
  saved state identity. Search refresh hanya saat effective Apply; logical
  Find/Go-to context tetap. Existing focus handler fokus input draft dan restore
  Source/query context saat selesai. Physical query/Go-to focus masih manual.
- Full parse/reconstruction/history O(n), bukan speedup. History200entries/8MiB,
  Source/native typing barrier, save identity dan export pipeline tidak diubah.

## Implementasi dan tes

`preview_paragraph::{Target,payload_range,payload,plan}` bounded lexical helper,
**bukan parser/authentication**. Projection `Block::paragraph_payload` hanya
root Kind::Paragraph depth0 dan physical-line eligible. Action adapter membawa
snapshot/ranges. Document mengautentikasi warm exact root sebelum scanner dan
candidate parser preflight sebelum commit `EditKind::PreviewParagraph`.

New draft/message/controller/UI terpisah dari cell draft agar grammar/value
validation tidak tertukar. Hylite whitelist/source edit guard/focus/view/title
menangani modal `[paragraph draft]`; no runtime hooks. Existing source reveal
masih tersedia; tidak ada shortcut baru/free Preview typing/selection formatting.

**26 Rust baru**, total Cargo **801** =775 lama +5 planner +2 history
+7 Document +3 action/model +8 controller +1 UI adapter. **277 strict core** subset
Cargo, bukan tambahan277 tes unik.

- Planner raw Unicode/markup/entity/escaped pipe/trailing spaces/prefix/suffix/LF/
  caret,16KiB byte bound/whitespace/control/multiline/EOF, forged snapshot/ranges/
  UTF-8/overflow, unchanged, fenced/heading lexical-positive counterexample.
- History atomic paragraph edge tags/Undo/Redo/Source barriers/no-op cannot retag/
  source typing does not merge into paragraph.
- Document exact raw prefix/suffix/neighbor preservation/one-step Undo/Redo/caret/
  cache, cold/nine forged/stale/replace, no-op and structural preflight refusal
  preserving redo/revision/render key/caret/cache/checkpoint, nested/literal/
  heading/multiline/indent/EOF refusal, fresh targets/inline changes, delayed save
  ack/branch/native barrier, exact16KiB/Unicode/raw inert inline literal.
- Action/model full snapshot/block/payload, root-only metadata, unsupported/nested/
  multiline/indent/EOF/heading/literal, separated roots and over-budget no action
  while source reveal remains available.
- Controller draft staging/Cancel/title/context, Preview/Split atomic Apply/caret/
  history, unchanged closes preserving redo, invalid/structural/oversized retention
  and correction, Source/prompt/Open/cell/no-draft guards, modal commands blocked,
  immutable Save/Export/replies, Find/Go-to logical context, stale/native barrier.
- UI adapter valid input +no error to enable Apply; parser preflight still Document.

Python **51** =49 lama +2 new independent literal raw draft/
Source/preflight refusal/caret/click/Undo/no-refocus/close and exclusive dispatch
probes. Private DISPLAY guard14suite sebelum subprocess.

## Suite GUI privat keempat belas

`--suite paragraph-draft` punya private app/Xvfb/XDG/env/clipboard sendiri,
alarm180s/step5min sendiri, job45min tetap. Semua24 old exercise bodies (12
regression phases +12 independent suites) AST-identical terhadap `7c3f84d`.
Ketiga belas gate lama **116+9+9+9+14+16+11+11+11+11+11+11+16** utuh tanpa
perubahan bytes/koordinat/deadline/refocus/oracle. No retry/runtime hooks.

Aktual **15 checks** =14 exact Copy (11 Source +3 draft) +natural clean Close0:

1. Welcome/fixture raw Unicode/markup/entity/escaped pipe/trailing spaces/terminal tab.
2. Open paragraph, raw payload copy dengan native input focus; Cancel mempertahankan
   seluruh Source dan tidak menambah history.
3. Unicode clipboard draft tetap staging; explicit Enter Apply mempertahankan LF,
   heading/suffix/raw bytes luar payload. One-step Preview Undo/Redo exact Source.
4. Split unchanged Apply menutup tanpa edge tambahan. `# heading` Enter mempertahankan
   draft/title; copy exact raw draft membuktikan parser menolak conversion sebelum
   commit. Cancel mempertahankan Source.
5. Split final Apply lalu native X tanpa Source refocus tepat di payload end
   (`finalX`), bukan suffix atau awal. Typing/Apply/paste history terpisah hingga
   welcome clean; natural client Close0.

Lima klik hanya explicit Edit paragraph product controls, bukan Source/input
refocus workaround. Screenshot `paragraph-draft/paragraph-preview.png`,
`paragraph-invalid.png`, `paragraph-split.png` bukan pixel/glyph oracle. Total aktual
**270 GUI** =243 clipboard (225 Source +18 draft),12 confirmation titles,15 natural
exits. Old source bar height/legacy fixture positions tetap diuji oleh gate lama;
ini bukan klaim semua previous paragraph source hit regions identik (Fill shrinks).

```sh
./scripts/test-gui-x11.sh --suite paragraph-draft --artifacts "$(mktemp -d)"
# Jalankan ketiga belas suite lama juga, bukan substitusi.
```

## Status eksekusi

Lokal: Rust/harness format, **277 strict core**, **51 Python**, diff check lulus.
[CI 39879922775](https://github.com/Indrawan007/Hylite/actions/runs/39879922775),
commit `56865a77e4b57325151a6b5d9d122a788b4bb65b`, **success**, watch exit0,
job `120174713822`,23 Oktober2026 00:45:10–00:53:15 UTC /08:45:10–08:53:15 Makassar.
Format/strict Clippy/native debug build, **801 Cargo**, **277 strict core**
(subset), **51 Python** dan **270 GUI** dalam14suite lulus. Annotation memuat semua
15 PASS paragraph dan totals semua suite lama. Artifact `12470087487`,
`gui-x11-smoke`,2.811.471 bytes, belum expired saat diperiksa; screenshot belum
direview visual. No retry/runtime hooks/refocus workaround/oracle weakening.
CI docs6p39879317480 cancelled saat push7a supersede; feature6p39878237851 tetap
success dan seluruh13 old gates kembali lulus di7a.

## Batas/manual

- [ ] Multiple root paragraphs/prefix/header neighbors/trailing spaces/16KiB UI,
  structural refusal variants selain heading, query/Go-to physical focus, narrow
  window/accessibility/keyboard traversal dan Source-bar hit regions lain.
- [ ] Glyph/caret/selection/scroll visuals, IME/RTL/HiDPI, Arch/Hyprland/Wayland/
  portal/IPC/GPU. X11 tiny-skia tidak menggantikan compositor target.

Belum multiline/EOF paragraph editing, block type conversion/create/delete/reorder,
free inline Preview typing/rich selection formatting, multi-block selection,
glyph↔caret mapping, synchronized scroll, incremental parse/virtualisasi/speed-RAM
benchmark. Source canonical LF/gutter tertunda/startup6b exit101 lama yang belum
terdiagnosis tetap; fitur ini bukan klaim memperbaiki startup/layout pixel.
