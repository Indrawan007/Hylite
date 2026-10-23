# Stage 7f — atomic multiline Apply & Previous

Melanjutkan [Apply & Next7e](testing-stage7e.md). Tombol **Apply & Previous**
menerapkan satu paragraf multiline root lalu membuka eligible multiline root
sebelumnya. Previous/Next unchanged, Apply-and-close, Cancel dan local Undo7c tetap.
Tidak menambahkan shortcut Apply, wrap, implicit creation, atau Apply & Previous
untuk sel tabel (sudah ada sejak6p).

## Kontrak

- Target awal tetap exact warm current root7b; eligible2–128baris nonempty/unindented,
  payload≤16KiB, LF final. Previous mengikuti urutan Source7d dan melewati heading/
  single-line/nested/code/literal/unsupported tanpa wrap atau block creation.
- **Boundary root pertama menolak sebelum Apply**, changed maupun unchanged. Tombol
  disabled bila tidak ada previous/error; direct message hanya notice dan draft/
  history/Source utuh. Apply biasa tetap tersedia untuk paragraf pertama.
- Changed Apply & Previous memakai **full candidate preflight** yang sama: edited
  range harus tetap complete root multiline Paragraph. Root **sebelum** payload yang
  diedit mempertahankan **offset byte aslinya** (editan berada sesudahnya), tetapi
  tetap diverifikasi ulang pada model kandidat: block/payload exact, payload sama
  persis, dan teks mentahnya identik. Tidak ada pergeseran delta untuk arah Previous;
  Next7e tetap memakai delta UTF8.
- Two-phase opaque plan yang sama: prepare read-only (candidate parse +native Source
  Content/caret rekonstruksi) lalu commit re-auth anchor snapshot/ranges **dan full
  Source equality**. Gagal/abandoned/stale/dokumen lain dengan teks berbeda tidak
  mengubah Source/history/cache. Kandidat sekali pakai; provisional target tak live.
- Commit sukses = satu native swap +shared `record_edit(PreviewParagraph)`
  (**satu edge atomik**), model kandidat dipasang warm, target distamp snapshot final.
  Sesi previous baru: raw payload exact, caret0, local Undo/Redo/error/notice kosong,
  neighbor flags current (Next ke root yang baru diedit).
- Source caret tetap di **akhir payload yang diterapkan**; Cancel draft previous tidak
  meng-Undo Apply sebelumnya. Unchanged Apply & Previous hanya pindah (tanpa revision/
  history/cache/checkpoint/redo/caret/search), tetapi membuka sesi lokal baru sehingga
  redo draft lama dibuang seperti7d/7e.
- Invalid/parser/structural/stale/native-init refusal mempertahankan draft mentah,
  selection, local history; scope focus restored. Oversize paste tetap ditolak boundary
  lokal7c. Source Undo/Redo/PreviewHistory tetap blocked selama modal; Ctrl+Z lokal
  kosong tidak menyentuh Source. Mode/prompt/Open/modal guards, immutable Save/Export
  token+reply, checkpoint identity, logical Find/Go-to dan Source barrier unchanged.
- UI: satu tombol160×32 **Apply & Previous** pada baris baru di bawah baris Previous/
  Next/Apply & Next, sehingga semua koordinat kontrol lama (termasuk Apply & Next7e)
  tidak bergeser; footer turun44px. Tidak ada shortcut/refocus hack baru. Klik tombol
  disabled bisa blur; Copy-before-click→Escape bukan proof fokus disabled.

## Implementasi / tes

`prepare_multiline_paragraph_next/_previous` kini membungkus satu helper internal
berparameter arah; tipe opaque dinamai `MultilineMovePlan` (sebelumnya
`MultilineNextPlan` di7e) dengan getter `move_target/move_value/neighbors`, dan
`commit_multiline_paragraph_move` menangani kedua arah. Kode Apply/Next/navigasi lama
tidak berubah perilaku. Copy full Source/parse/dua buffer native O(n) +memori ekstra;
bukan klaim incremental/performance/RAM. Peak plan menyimpan prefix+trailing Source.

**903 Cargo** =886sebelumnya +**17Rust** (**8Document +7controller +2UI**);
**294strict core** subset unchanged, **61Python** =59+2.

- Document: previous exact ranges/payload dan raw bytes, fresh snapshot/model/caret,
  atomic Undo/Redo, noop mempertahankan pointer/caret/checkpoint/Source redo, boundary
  pertama, lexical/control/structural refusal, cold/eight forged/single/nested/literal/
  heading targets, abandoned/cold/stale plan, dokumen lain dengan Source berbeda
  (ranges/payload masih cocok) ditolak, sequential dua edge +Source typing barrier,
  Save ack/caret boleh menyela tetapi replace ditolak, UTF8 grow/shrink/byte/128baris
  dengan offset previous tetap.
- Controller: Preview/Split commit kedua → draft pertama fresh (caret0/history kosong/
  Next tersedia), caret Source di akhir payload applied, Undo kembali ke Source awal;
  local Undo→unchanged transition menjaga Source redo/caret; boundary pertama notice +
  Apply tetap valid; lexical/structural/oversized/stale refusal menjaga local history;
  Source/pending/Open/no-draft/partial-modal guards; navigasi Next setelah Apply &
  Previous memakai payload applied; Source typing tetap barrier; immutable Save/Export
  reply +logical search/Go-to refresh.
- UI: affordance enabled untuk draft valid ber-notice saat previous ada; disabled pada
  boundary/error/invalid; native previous buffer exact caret0/history kosong dibangun
  sebelum Source berubah. Semua tes lama tetap.

## Private GUI suite19

`--suite paragraph-apply-previous`: private app/Xvfb/XDG/env/clipboard, alarm180s,
step5menit sendiri, job45menit tetap. **28old exercise bodies AST-identical terhadap
`bb37223`** (28 termasuk suite7d/7e); 18gate lama dipertahankan tanpa perubahan bytes/
koordinat/oracle/deadline/refocus, tanpa retry/runtime hook. Private DISPLAY guard
kini19suite.

**26checks** =25Copy(10Source+15draft)+natural clean Close0: fixture tiga root raw
Unicode dengan heading/single/quote/fence/list di antaranya; open first, dua Next
unchanged ke root ketiga; edit tiga baris Unicode, Apply & Previous mengganti hanya
root ketiga lalu membuka middle exact (focus/native offsets baru); local Ctrl+Z kosong;
unchanged Apply & Previous ke first tanpa Source edge; draft first changed lalu klik
boundary pertama disabled; Copy dilakukan **sebelum** klik disabled karena klik tombol
disabled memblur input (bukan workaround refocus); Escape lalu Cancel membuktikan Source
tidak berubah; satu Source Undo/Redo; Split: middle structural
`one\n# heading` refusal lalu Cancel menjaga Source; Split final middle replace →
Apply & Previous membuka first; Escape lalu X tanpa refocus membuktikan caret di akhir
payload middle yang diterapkan (bukan first); typing/dua Apply & Previous Undo
bertahap kembali ke fixture lalu welcome clean; natural Close0.

12klik hanya produk Open/Next/Apply & Previous (termasuk satu disabled boundary).
Screenshot `paragraph-apply-previous-refused.png` dan `-split.png` bukan pixel oracle.

```sh
./scripts/test-gui-x11.sh --suite paragraph-apply-previous --artifacts "$(mktemp -d)"
# Keep all18old suites too, never substitute.
```

## Execution status

Local fmt/harnessfmt/294strict core/61Python/diff/AST check PASS.

[CI 39892699395](https://github.com/Indrawan007/Hylite/actions/runs/39892699395),
`0353f5e8363b6be9b63ed235b1fa15e5f869a2ef`, job120215906995 **SUCCESS**,
2026-10-23 **03:00:56–03:11:10 UTC** / **11:00:56–11:11:10 Makassar**. Format,
strict core subset, Clippy, build, **903Cargo**, seluruh **19suite** (**378GUI**,
26checks suite baru) dan upload artifact lulus. Artifact `12475384207`,
**3.279.313bytes**, SHA256
`92636145659d325378e7c3073665a601d74c93bb1dcfa1bc6b1f95f4574b06a2`, expires
2026-10-30T03:11:05Z, unexpired. Native counts dari check annotations; screenshot
tidak direview visual dan full archive tidak diperiksa.

Iterasi CI sebelum lulus, semuanya test-only setelah produk `2d33fea`:
`39889996740` gagal `unused_mut`; `39890327566` empat assertion tes baru (prefix
Source, target valid yang dikira stale, fixture read-only, offset lintas Document);
`39890515613` satu assertion (Source pembanding foreign ternyata identik);
`39890717797` suite baru berhenti setelah10checks karena Copy **sesudah** klik
disabled yang memblur input; `39891676944` satu assertion judul dirty yang salah
sebelum fixture. Tidak ada pelonggaran lint/bytes/koordinat/oracle/deadline/refocus
dan tidak ada perubahan produk untuk membuat tes lulus.

## Manual / limits

- [ ]Banyak root, dokumen besar/offscreen, transisi pada128baris/16KiB, native
  failure UI, selection direction, disabled blur/keyboard/narrow window/accessibility,
  physical query/Go-to focus dan scroll/layout/glyph.
- [ ]Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU. X11 tiny-skia bukan proof
  compositor target; screenshot tidak direview visual.

No cross-kind navigation/EOF/blank/indented editing, implicit Apply/create/delete/
reorder, Apply shortcut, rich selection/inline WYSIWYG/glyph mapping/multi-block
selection/sync scroll/incremental parse/performance claim. Source remains
authoritative; full candidate preflight, bounded Source history, immutable file
pipeline preserved.
