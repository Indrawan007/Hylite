# Stage 6p — Apply & Previous in the raw cell draft panel

Successor [column duplication6o](testing-stage6o.md); complements the existing
[Previous/Next and Apply & Next6i](testing-stage6i.md). Explicit one-cell Apply
plus bounded backward navigation, not spreadsheet/inline WYSIWYG.

## Kontrak produk

- **Apply & Previous** adalah tombol keempat pada navigation row panel draft,
  setelah Previous, Next dan Apply & Next. Width160/height32; posisi/width tiga
  tombol lama, input, Apply/Cancel dan main Preview/Split picker tidak berubah.
  Tanpa shortcut baru atau perubahan menu Table actions.
- Terapkan payload sel saat ini sebagai **satu Atomic PreviewTable**, lalu buka
  payload real sebelumnya secara row-major dalam tabel yang sama. Header row0,
  body mulai1; Previous dari body pertama melintasi delimiter ke header terakhir.
  Delimiter bukan sel editable. Tidak wrap/lompat tabel/membuat baris atau kolom.
- Pada sel pertama, Apply & Previous disabled. Forged command menolak tanpa
  menerapkan perubahan, termasuk draft berubah. Gunakan Apply/Enter untuk
  menyelesaikan sel pertama; Cancel/Escape membuang hanya draft saat ini.
- Draft valid dan unchanged boleh berpindah tanpa revision/history/cache/caret/
  checkpoint mutation atau redo clearing. Effective Apply selalu satu edge;
  rangkaian mundur tidak digabung menjadi satu session Undo.
- Input invalid/oversized/stale atau preflight gagal tidak diterapkan sebagian.
  Valid dirty draft boleh memakai Apply & Previous; Previous biasa tetap menolak
  dirty draft. Error dan navigation notice tetap terpisah.
- Semua provenance/batas6h tetap: warm current root membership, full snapshot/
  table range/row/column/dimensions/exact payload; canonical rectangular edge-pipe
  real cells, maksimal12kolom/128row/512sel, single-line16KiB raw payload, valid
  escaped pipes/backslash parity/control rules. EOF tanpa LF mengikuti eligibility
  sel6h, bukan pembatasan full-line column actions6k. Cold/stale/forged/nested/
  padding/fallback tidak di-rebase atau diautentikasi hanya oleh lexical neighbor.
- Effective transition mem-preflight **full candidate Source parse sebelum
  commit**: current root start/dimensions dan real previous-cell metadata harus
  resolve. Setelah satu commit, candidate cache dipasang dan previous target
  distamp full Snapshot baru. No-op memakai current authenticated cache tanpa full model parse.
- Source caret tetap **di akhir payload yang baru diterapkan**, bukan berpindah ke
  payload sebelumnya atau glyph. Panel input mendapat focus melalui existing
  handler; Apply/Cancel terakhir mempertahankan mode dan memulihkan Source/query
  context seperti6h. Cancel tidak membatalkan Apply sebelumnya.
- Source tetap authority. Active draft memblokir dokumen/file/history/mode command
  lain; Source mode/pending confirmation/Open menolak Apply. Running Save/Export
  token tetap immutable snapshot awal; delayed ack mengikuti saved state identity.
  Search direfresh hanya untuk effective Apply; logical Find/Go-to context tetap.
- Full parse/reconstruction/history tetap O(n), bukan benchmark/speedup. History
  budget200entries/8MiB dan Source/native typing barrier tidak berubah.

## Implementasi dan tes

Message `ApplyPreviewCellPrevious` ada di whitelist draft dan dispatch ke existing
controller transition pipeline yang diparameterkan Direction. Status/error lama
Apply & Next tetap. Document menyediakan `edit_preview_cell_and_previous` dan
mempertahankan API `edit_preview_cell_and_next`; keduanya memakai private shared
preflight helper. No new buffer/AST serialization/runtime hooks. UI enable helper
hanya bounded affordance, bukan current-root authentication.

**16 Rust baru**, total Cargo **775** =759 lama +7 Document +7 controller
+2 UI adapter. **270 strict core** tetap, subset Cargo, bukan tambahan270 tes unik;
resolver portable/Source planner tidak berubah.

- Document raw Unicode/escaped pipe/body→header sequence, fresh previous snapshot/
  payload/cache, Source caret at applied end, separate one-step Undo/Redo; unchanged
  preserving revision/render key/caret/checkpoint/redo;12 forged/cold/stale/replace/
  invalid/oversized reject-all; first-cell changed/unchanged no-wrap; header-only/
  single/empty/indent/12kolom/exact512sel; exact inner fence/nested/ragged/HTML/
  fallback; delayed save ack/Source barrier/history branch identity.
- Controller Preview/Split/fresh draft/Cancel-current-only; no-op and first-cell
  refusal preserving draft/redo without poisoning valid Apply; invalid/oversized/
  stale retention; Source/prompt/Open/no-draft/Save/Export guards/tokens; sequential
  effective edges/search refresh/logical Find/Go-to; native typing barrier; active
  draft blocks document commands and delayed save ack remains immutable.
- UI adapter locks first-cell/invalid disable, valid dirty/backward/header-only and
  navigation-notice enable without discarding draft. Existing UI tests untouched.

Python **49** =47 lama +2 new independent literal raw draft/Source/alignment/caret/
click/Undo/no-refocus/close and exclusive dispatch probes. Private DISPLAY guard
mencakup13suite sebelum subprocess.

## Suite GUI privat ketiga belas

`--suite cell-apply-previous` memakai private app/Xvfb/XDG/env/clipboard sendiri,
alarm180s/step5min sendiri, job45min tetap. Semua23 old exercise bodies (12
regression phases +11 independent suites) AST-identical terhadap `803ff5c`.
Dua belas gate lama **116+9+9+9+14+16+11+11+11+11+11+11** utuh tanpa perubahan
bytes/koordinat/deadline/refocus/oracle. Tanpa retry/runtime hooks.

Aktual **16 checks** =15 exact Copy (9 Source +6 draft) +natural clean Close0:

1. Welcome/fixture raw UTF-8, markup, escaped pipe/entity, alignment/indent/tab/suffix.
2. Last body raw draft exact; invalid input exact sebelum disabled Apply & Previous.
   Klik disabled, title tetap draft; Cancel lalu Source Copy membuktikan tidak Apply.
3. Reopen melalui picker produk; valid Unicode Apply & Previous membuka preceding
   raw body payload dengan native draft focus. No-op dua kali melintasi delimiter
   dan header hingga Name, tanpa tambahan history.
4. Modified first-cell draft exact sebelum disabled backward boundary; klik tetap
   pada title first-cell. Cancel membuang draft itu saja; Source byte-exact hanya
   memuat Apply body sebelumnya. Ini bukan klaim input focus setelah disabled click.
5. Satu Preview Undo/Redo memulihkan Source sebelum/sesudah effective Apply.
6. Split Apply & Previous, Cancel, native X tanpa Source refocus masuk tepat pada
   applied payload end (`finalX`), bukan ke previous payload; separate typing/Apply
   Undo, undo seluruhnya sampai welcome clean tanpa hidden edge, natural Close0.

Klik hanya picker/opsi/tombol produk, bukan Source/input refocus workaround. Disabled
click secara native dapat melepas input focus; tidak menyamarkan lewat klik fokus
atau oracle keyboard setelah disabled button. Cancel/reopen adalah bagian eksplisit
workflow invalid-draft, bukan retry. Screenshot
`cell-apply-previous/cell-apply-previous-draft.png` / `cell-apply-previous-split.png`
bukan pixel/glyph oracle. Total aktual **255 GUI** =229 clipboard (214 Source
+15 draft),12 confirmation title,14 natural exits.

```sh
./scripts/test-gui-x11.sh --suite cell-apply-previous --artifacts "$(mktemp -d)"
# Tetap jalankan dua belas suite lama; ini bukan substitusi.
```

## Status eksekusi

Lokal: Rust/harness format, **270 strict core**, **49 Python**, diff check lulus.
[CI 39878237851](https://github.com/Indrawan007/Hylite/actions/runs/39878237851),
commit `6caedff3615deedfb5217fcace6f8c7974595aad`, **success**, watch exit0,
job `120169213439`,23 Oktober2026 00:28:12–00:36:51 UTC /08:28:12–08:36:51 Makassar.
Format/strict Clippy/native debug build, **775 Cargo**, **270 strict core**
(subset), **49 Python** dan **255 GUI** dalam13suite lulus. Annotation memuat semua
16 PASS Apply Previous dan totals semua suite lama. Artifact `12469403519`,
`gui-x11-smoke`,2.622.926 bytes, belum expired saat diperiksa; screenshot belum
direview visual. Tidak ada retry/runtime hooks/refocus workaround/oracle weakening.
CI docs6o39877584000 cancelled ketika push6p supersede; feature6o39876692641
sudah success dan semua dua belas gate lama kembali lulus di6p.

## Batas/manual

- [ ] Header Apply & Previous/EOF/empty/single/12kolom/exact512sel UI, query/Go-to
  physical focus, narrow window/accessibility/keyboard traversal dan glyph/caret/
  selection/scroll visuals. Unit guard bukan pixel proof.
- [ ] Arch/Hyprland/Wayland/portal/GPU, IME/RTL/HiDPI. X11 tiny-skia tidak membuktikan
  compositor target, file portal, IPC atau renderer GPU.

Tidak ada general inline Preview typing, rich selection formatting, multi-cell
selection, implicit Apply, draft-session Undo, glyph↔caret mapping, scroll sync,
incremental parse/virtualisasi/performance benchmark. Source canonical LF/gutter
tertunda/startup6b exit101 lama yang belum terdiagnosis tetap; fitur ini bukan
klaim memperbaiki startup/layout pixel.
