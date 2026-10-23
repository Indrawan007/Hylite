# Stage 6o — duplicate one raw column with alignment

Successor [column movement6n](testing-stage6n.md). Explicit duplication, not drag,
bulk/multi-column selection, resize, implicit cell Apply or general WYSIWYG.

## Kontrak produk

- **Table actions… → Column N: Duplicate**, N mulai1. Satu kolom disalin ke
  **sebelah kanan kolom asal** pada header, delimiter dan seluruh body sekaligus.
  Semua opsi baru append **sesudah seluruh prefix6n**, termasuk row/column moves.
  Width168/text11/action-bar height, label/index/shortcut lama tidak diubah.
- Copy **raw segment termasuk right separator** setiap physical line. Raw spaces/
  tab/Unicode/markup/escaped pipes/entity dan delimiter dash/alignment ikut asal;
  row indent, bytes setelah outer pipe, kolom lain, prefix/suffix tetap. Tidak ada
  AST serialization/normalization. Contoh `| Name | Count |` Duplicate kolom2 →
  `| Name | Count | Count |`; `| --- | --: |` → `| --- | --: | --: |`.
- Satu whole-table TextChange = **satu Atomic PreviewTable edge**. Candidate full
  Source parse sebelum mutation harus mempertahankan complete root range,
  same row count, column count+1, real canonical cells/delimiters. Refusal tidak
  mengubah Source/revision/history/redo/checkpoint/caret/cache; tidak partial copy.
- Source caret menuju **awal payload header kolom salinan**, bukan kolom asal
  atau body. Split menggunakan existing focus handler bila Find/Go-to tidak aktif;
  native typing berikut menjadi transaksi/barrier sendiri.
- Canonical rectangular root eligibility6k, warm model dan exact target snapshot/
  root range/dimensions/selected header/marker wajib. Full physical header/
  delimiter/body outer-piped LF; raw cells valid6h, maksimal16KiB payload.
  Ragged/padding/extra cells, nested/fence/HTML/fallback, EOF tanpa LF dan missing
  outer pipe ditolak. Lexical planner saja bukan provenance authentication.
- Header-only, single-column dan empty payload boleh. **Identical/empty cells
  tetap menambah satu kolom**; ini bukan no-op seperti swap identik6n.
- Growth guard sama6k: maksimal12kolom/128row termasuk header/512sel. Current
  rows×(columns+1) harus muat;128×3 →128×4 boleh,128×4 →128×5 ditolak. Row cap
  sendiri tidak melarang karena tidak menambah baris. Kolom12 tidak bisa duplicate.
- Source mode, pending confirmation/Open dan active cell draft menolak command;
  draft dipertahankan, tidak di-Apply implicit. Running Save/Export token tetap
  immutable snapshot awal; save ack mengikuti history state identity.
- Source tetap authority. Projection lama diinvalidasi saat effective commit;
  target/cell berikut harus fresh. Search matches di-refresh termasuk matches
  baru di kolom salinan; logical Find/Go-to context tetap. Full parse/reconstruction/
  history O(n), bukan speed/RAM improvement.

## Implementasi dan tes

`preview_table_column::Action::Duplicate` menggunakan existing `can_insert` dan
backslash-parity scanner. Copy selected segment tepat setelah dirinya per line,
resolve copied header caret melalui canonical replacement cells. Document reuse
`insertable_columns` authentication dan full candidate preflight columns+1.
Existing controller/modal/file/history/search/focus pipeline tidak diganti.

**21 Rust baru**, total Cargo **759** =738 lama +6 planner +7 Document
+3 picker/model +5 controller:

- Planner first/last/middle raw copy, alignment/dash/Unicode/escaped pipes/entity/
  markup/spacing/indent/tab/prefix/suffix/new header caret; single/empty/header-only/
  identical, exact growth bounds/overflow; forged snapshot/ranges/dimensions,
  malformed/ragged/EOF; fenced lexical-positive bukan root authentication.
- Document atomic exact bytes/alignment/dimensions/caret/cache/history;12 forged
  cases/cold/stale/replace; capacity refusal preserving redo/checkpoint/caret/cache;
  header-only/single/empty/identical/indent/exact512sel; nested/literal/ragged/
  noncanonical/EOF/fallback; fresh copied cell Apply independent from original;
  delayed save ack/branch/Source barrier.
- Picker entire22 old label prefix tepat, appended Duplicate1/2 full targets and
  Message, unsupported/nested/growth cap suppression; single/empty/header-only/
  exact cell capacity. Existing fixture totals22→24, old prefix assertions utuh.
- Controller Preview/Split both columns/caret/one-step Undo/Redo; Source/prompt/
  Open/modal/Save/Export tokens; stale/redo/native barrier; search new duplicated
  match offsets +logical Go-to; save ack/exact growth refusal/checkpoint.

**270 strict core** subset Cargo, bukan tambahan270 tes unik. Python **47**
menambah2 independent raw copy/alignment/new-header caret/four-click/Undo/no-refocus/
exit and exclusive dispatch tests. Private DISPLAY guard mencakup12suite sebelum
subprocess; no runtime hooks.

## Suite GUI privat kedua belas

`--suite table-column-duplicate` punya private Xvfb/app/XDG/env/clipboard sendiri,
alarm180s/step5min, job45min tetap. Sebelas gate lama **116+9+9+9+14+16+11+11+11+
11+11** utuh. Semua22 old exercise bodies (12 regression phases +10 independent
suites) AST-identical terhadap `4e96cd2`. Tidak mengubah bytes/koordinat/refocus/
deadline lama; tanpa retry/refocus workaround/oracle weakening.

Aktual **11 checks** =10 exact Source Copy +natural clean client Close exit0:

1. Welcome/fixture raw Unicode/markup/escaped pipe/entity/spacing/terminal tab.
2. Preview Column1 Duplicate menyalin Name/header/delimiter/alignment/body utuh.
3. Satu Preview Undo menghapus full duplicate; Redo kembali exact copied columns.
4. Undo sebelum Split memulihkan original Source dan old action menu prefix.
5. Split Column2 Duplicate lalu native X tanpa refocus menuju copied third header:
   `| Name | Count | XCount |`, bukan original header/body.
6. Undo typing menyisakan full duplication, Undo duplicate kembali original,
   Undo paste kembali welcome clean tanpa hidden edge; natural client Close0.

Empat klik hanya picker/opsi produk. Header-only/single/empty/growth cap guards
diuji unit, bukan runtime pixel/menu absence oracle. Evidence
`table-column-duplicate/column-duplicate-preview.png` / `column-duplicate-split.png`
bukan glyph/pixel oracle. Total aktual **239 GUI** =214 clipboard (205 Source +9
draft),12 title,13 natural exits.

```sh
./scripts/test-gui-x11.sh --suite table-column-duplicate --artifacts "$(mktemp -d)"
# Jalankan sebelas suite lama juga; ini bukan substitusi.
```

## Status eksekusi

Lokal: Rust/harness format, **270 strict core**, **47 Python**, diff check lulus.
[CI awal 39876046796](https://github.com/Indrawan007/Hylite/actions/runs/39876046796)
pada `074204d` lulus format/Clippy/build dan758 dari759 Cargo, tetapi satu **tes
baru** controller capacity gagal: helper fixture lama mengasumsikan2×2 sementara
fixture ini128×3. GUI skipped, bukan lulus. `9465436` memperbaiki helper lokal
hanya pada tes baru dengan actual fixture dimensions sebelum/sesudah duplicate
(128×3/128×4), menambah exact copied header/delimiter prefix dan dimensions
assertion; tidak mengubah production atau oracle lama.

[CI ulang 39876692641](https://github.com/Indrawan007/Hylite/actions/runs/39876692641),
commit `94654369f44fd06aa2c2bd585f112b2b144bfcd1`, **success**, watch exit0,
job `120163898031`,23 Oktober2026 00:12:21–00:19:28 UTC /08:12:21–08:19:28 Makassar.
Format, strict Clippy, native debug build, **759 Cargo**, **270 strict core**
(subset), **47 Python** dan **239 GUI** dalam12suite lulus. Annotation memuat semua
11 PASS duplicate dan totals semua suite lama. Artifact `12468243673`,
`gui-x11-smoke`,2.532.220 bytes, belum expired saat diperiksa; screenshot belum
direview visual. Tidak ada retry/oracle weakening/runtime hooks/refocus workaround.
CI docs6n39875502116 cancelled ketika push6o supersede; feature6n39874500418
sudah success. Semua gate6n dijalankan lagi pada6o.

## Batas/manual

- [ ] Middle/3+kolom/single/empty/header-only/11→12kolom/3spasi/trailing-tab UI,
  physical active Find/Go-to focus, large menu keyboard/scroll/accessibility dan
  growth-capped menu absence. Unit guard bukan pixel proof.
- [ ] Glyph/caret/selection/scroll visuals, IME/RTL/HiDPI dan target desktop
  Arch/Hyprland/Wayland/portal/GPU. X11 tiny-skia bukan verifikasi compositor.

Tidak ada bulk/drag/multi-column selection/resize, whole-table deletion, implicit
Apply/draft session Undo, free Preview typing/rich selection formatting, glyph↔caret
mapping, scroll sync, incremental parse/virtualisasi/performance benchmark. Source
canonical LF/gutter tertunda/startup6b exit101 lama yang belum terdiagnosis tetap;
fitur ini bukan klaim memperbaiki startup atau layout pixel.
