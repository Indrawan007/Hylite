# Stage 7j — navigasi dan Apply & Previous/Next paragraf satu baris

Melanjutkan [konteks draft7i](testing-stage7i.md). Sampai7i panel paragraf satu
baris hanya punya `Apply`/`Cancel`: untuk pindah ke paragraf lain pengguna harus
`Cancel`, klik teks Preview lagi, lalu `Apply` lagi. Stage7j memberi paritas dengan
draft multiline7d–7f tanpa memperluas jenis blok yang boleh diedit:

1. `Previous paragraph`/`Next paragraph` — pindah draft **hanya bila teks belum
   berubah**; draft kotor menampilkan notice dan tetap utuh.
2. `Apply & Previous`/`Apply & Next` — satu langkah: commit atomik payload yang
   sekarang, lalu buka payload raw root tetangga sebagai draft fokus baru.

Root yang eligible tetap sama seperti7a/7b: blok `Kind::Paragraph` yang punya
`paragraph_payload` **dan tidak** punya `multiline_paragraph_payload`. Heading,
quote, code fence, list, dan root multiline dilewati; urutan mengikuti Source; tidak
ada wrap di ujung; tidak ada pembuatan paragraf baru.

## Kontrak

- **Fase rencana baca-saja.** `Document::prepare_single_paragraph_next|previous`
  (`&self`) memvalidasi target warm, nilai draft, dan tetangga, lalu:
  - menghitung kandidat Source `replace_range(payload, value)` lewat
    `preview_paragraph::plan` dan mem-parse **seluruh** kandidat;
  - membuktikan payload hasil tetap **tepat satu root paragraf satu baris** dengan
    rentang byte persis (`source == start..start+len+1`, payload `..end-1`);
  - menemukan blok tetangga di kandidat (offset digeser untuk `Next`, tidak berubah
    untuk `Previous`), memastikan blok itu ada di kandidat, dan byte payload-nya
    **byte-identik** dengan payload yang akan dibuka;
  - menghitung flag `[previous, next]` dari blok kandidat, provenance baris tetangga,
    dan native buffer `text_editor::Content` dengan caret di akhir payload hasil.
  Semua kegagalan mengembalikan `None`; belum ada byte Source, snapshot, cache, atau
  history yang berubah.
- **Fase commit.** `Document::commit_single_paragraph_move` memeriksa ulang
  `preview_paragraph_payload(anchor) == original` **dan** `buffer_text() ==
  source_before` sebelum bertindak. Bila teks berubah: `break_undo_group`, `record`,
  `replace_content`, `record_edit(EditKind::PreviewParagraph)`, lalu cache preview
  diganti hasil parse kandidat yang sama; tepat **satu edge** history. Bila teks
  tidak berubah (`edit: None`) tidak ada edge, redo tetap hidup, snapshot/render key/
  caret identik. Snapshot target hasil di-retarget ke snapshot baru.
- **Urutan preflight controller** (`apply_preview_paragraph_adjacent`): guard mode/
  modal → draft ada → nilai valid & tanpa error → target belum basi → tetangga ada
  (kalau tidak: `navigation_notice`, `Apply` tetap aktif) → `prepare_*` → bangun draft
  tetangga **dari plan**, bukan dari dokumen yang belum berubah → `commit`. Penolakan
  mana pun mempertahankan draft dan tidak mengubah Source; hanya boundary memakai
  notice, sisanya `error`.
- Draft tetangga tidak pernah dibangun dengan offset pasca-edit pada dokumen lama:
  penggantian panjang dapat menggeser offset, sehingga byte/offset tetangga hanya
  otentik bila diambil dari parse kandidat. Kegagalan versi pertama (refusal palsu
  untuk `Next` saat panjang berubah) ditemukan oleh tes controller dan diperbaiki
  dengan menyimpan `line` hasil plan.
- `Ctrl+PageUp/PageDown`, `Ctrl+Enter`, dan `Enter` **bukan** rute commit di panel ini:
  `text_input` Iced0.13 menangkap `Enter`/`Escape`/`PageUp`/`PageDown` saat fokus.
  Karena itu Stage7j hanya menambah tombol, tanpa shortcut baru.
- Baris tombol baru tidak menggeser apa pun: header, provenance (20px), hint, input,
  serta baris `Apply`/`Cancel` tetap pada tinggi dan koordinat yang sama.

## Implementasi

- `document/state.rs`: `SingleMovePlan` (`anchor`, `original`, `source_before`,
  `moved`, `value`, `line`, `neighbors`, `edit: Option<PreparedSingleEdit>`) dengan
  akses `move_target`/`move_value`/`move_line`/`neighbors`;
  `adjacent_single_paragraph`; `prepare_single_paragraph_next|previous`;
  `commit_single_paragraph_move`. `SingleMovePlan` di-reexport lewat
  `document::SingleMovePlan` (crate-private).
- `app/paragraph_edit.rs`: `paragraph_draft_for` dipakai bersama oleh open/move;
  `move_preview_paragraph`, `apply_preview_paragraph_next|previous`, dan
  `decide_preview_paragraph_adjacent` yang mengembalikan `AdjacentDecision`
  (`Refused(AdjacentRefusal)` / `Proceed { plan, moved }`) sehingga tidak ada lagi
  pinjaman `&mut self.paragraph_draft` saat membaca dokumen.
- `ui/paragraph_edit.rs`: `Draft.navigation_notice`, `Draft.neighbors: [bool; 2]`,
  `can_move`, `move_message`, `apply_adjacent_message`, dua baris tombol baru
  (`Previous paragraph`/`Next paragraph` 160×32, `Apply & Previous` 160×32,
  `Apply & Next` 140×32) dan teks notice/error di footer.
- `commands.rs`/`app.rs`: `MovePreviewParagraph(ParagraphDirection)`,
  `ApplyPreviewParagraphNext`, `ApplyPreviewParagraphPrevious` masuk whitelist draft
  paragraf dan dispatch; guard7i tetap menangani gesture lain.
- **938 Cargo** = 921 sebelumnya + **17 Rust**: 8 dokumen (`preview_paragraph_navigation_tests.rs`),
  7 controller (`src/app/tests.rs`), 2 UI (`src/ui/paragraph_edit.rs`).
  **294 strict core** subset unchanged, **65 Python**.

Cakupan tes Rust:

- tetangga mengikuti urutan Source dan melewati heading/quote/fence/multiline; ujung
  pertama/terakhir `None`; pembacaan tidak mengubah snapshot/render key/caret/dirty/
  undo/redo/pointer cache;
- Apply & Next atomik + undo/redo; Apply & Previous menjaga offset root sebelumnya;
- unchanged move mempertahankan identitas, tanpa edge baru, dan redo tetap;
- target basi/forged/multiline/dokumen asing ditolak tanpa mutasi;
- dua edge berurutan, barrier ketikan, dan perilaku `restore_preview_history`;
- controller: nav di Preview & Split, penolakan draft kotor tanpa membuang draft,
  Apply & Next/Previous per mode, unchanged tanpa edge, guard invalid/stale/mode/
  tanpa-draft, serta koeksistensi Save/Export/Find/Go-to;
- UI: affordance butuh nilai valid, tetangga ada, dan tanpa error; notice tidak
  menutupi error otoritatif.

## GUI

Suite ke-21 `paragraph-navigation-single` (25 pemeriksaan: 24 clipboard + 1 exit
native) berjalan di Xvfb terpisah dengan alarm180s. Fixture: tiga root paragraf satu
baris (`café🙂 **first**  `, `second é🙂 &amp;`, `third 🙂  `) yang dipisahkan heading,
quote, code fence, dan list. Yang diperiksa: payload raw persis saat dibuka, `Next`
melewati blok tak didukung, replacement Unicode dua spasi tetap lokal sampai
`Apply & Previous`, `Apply & Previous` meng-commit payload ketiga lalu membuka root
tengah, `Apply & Previous` unchanged sampai root pertama tanpa edge, `Ctrl+PageUp`
bukan rute commit, tombol boundary pertama disabled tidak mengubah apa pun, satu
Undo menghapus tepat satu edge dan Redo memulihkannya, penolakan parser
(`# heading`) mempertahankan draft+fokus, `Apply & Next` membuka root tengah,
`Escape` mengembalikan fokus/caret Source di akhir payload yang baru diterapkan,
ketikan menjadi edge terpisah, dan empat Undo kembali ke welcome tanpa edge
tersembunyi. Dua screenshot tanpa asersi piksel.

Ke-20 suite lama (394 checks) dijalankan ulang utuh dan lulus; 28 badan exercise lama
tetap AST-identical `bb37223`, tanpa perubahan bytes/koordinat/oracle/deadline/
refocus. Tombol baru berada di baris tambahan di bawah baris `Apply`/`Cancel`
sehingga koordinat suite lama tidak berubah.

## Execution status

Local `cargo fmt --all -- --check`, harnessformat, **294 strict core**, **65 Python**,
`git diff --check`, dan pemeriksaan AST 28 badan exercise PASS.

[CI 39906647794](https://github.com/Indrawan007/Hylite/actions/runs/39906647794),
`ecb6693`, job120260038439 **SUCCESS**, 2026-10-23 **05:35:58–05:47:04 UTC** /
**13:35:58–13:47:04 Makassar**. Format, strict core subset, Clippy `-D warnings`,
build, **938 Cargo**, seluruh **21 suite** (**419 pemeriksaan GUI**: 385 clipboard —
289 Source + 96 draft input; 12 title konfirmasi; 22 exit native normal) dan upload
artifact lulus. Artifact `12481318417`, **3.315.303 bytes**, expires
2026-10-30T05:47:00Z; digest SHA256 tidak dapat diunduh dari sandbox ini (host
penyimpanan artifact tidak dijangkau), screenshot tidak direview visual.

Empat percobaan sebelumnya gagal dan diperbaiki secara jujur sebelum lulus: (1)
pinjaman `&mut self.paragraph_draft` vs `&self` di controller (`E0502`) serta 3 import
tak terpakai; (2) `move_line`/`line` salah tertempel pada `MultilineMovePlan`;
(3) draft tetangga dibangun dari dokumen yang belum berubah sehingga `Next` dengan
panjang berbeda ditolak palsu; (4) ekspektasi barrier history dan baris `title_is_dirty`
di suite GUI salah tempat. Tidak ada gate, byte, koordinat, oracle, atau deadline
yang dilonggarkan.

## Manual / limits

- [ ]Efek `Apply & Next` pada dokumen besar (parse penuh kandidat + dua buffer native)
  terhadap latensi/ memori; belum ada klaim performa.
- [ ]Perilaku tombol pada lebar jendela sempit, HiDPI, font non-default, IME/RTL.
- [ ]Arch/Hyprland/Wayland/portal/GPU. X11 tiny-skia bukan proof target.

Navigasi tetap terbatas pada paragraf satu baris ↔ paragraf satu baris; tidak ada
navigasi lintas jenis blok, tidak ada wrap, tidak ada paragraf baru, dan root
multiline tetap memakai panel7b–7f.
