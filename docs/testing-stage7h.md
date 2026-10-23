# Stage 7h — hardening properti/diferensial draft multiline

Melanjutkan [shortcut7g](testing-stage7g.md). Stage ini **tidak mengubah kode
produk**: seluruh perubahan berupa tes deterministik yang memeriksa ulang kontrak
7b/7d/7e/7f terhadap Source acuan independen. Tujuannya menutup celah kombinasi
input yang tidak mungkin ditulis tangan satu per satu, sekaligus mengunci perilaku
yang sudah lulus sebelum langkah berikutnya.

## Yang ditambahkan

`src/document/preview_multiline_property_tests.rs` (5 tes Cargo) memakai **xorshift
64-bit deterministik tanpa crate eksternal**, jadi setiap kegagalan dapat direproduksi
dari seed yang tertulis di berkas. Korpus sumber mencakup paragraf multiline Unicode,
hard break, escaped pipe, entity, heading, single line, quote, code fence, list,
tabel GFM, baris kosong, dan tab terminal.

- **Commit vs acuan**: 400 kasus sumber+target+nilai (Next/Previous) dibandingkan
  dengan transformasi acuan `replace_range(payload, value)` pada Source asli.
  Diperiksa: buffer native sama persis, `changed` sesuai, caret Source = akhir payload
  yang diterapkan, hasil parse tetap **satu root multiline Paragraph** dengan rentang
  off­set tepat, neighbour bergerak mempertahankan payload persis, satu Undo kembali
  ke Source asli dan satu Redo memulihkan hasil, sedangkan perpindahan unchanged
  tidak menambah edge maupun mengubah identity. Ambang cakupan: >40 commit,
  minimal satu refusal.
- **Refusal murni**: 250 kasus dengan nilai tidak valid (kosong, satu baris, indentasi,
  blank line, tab, heading, fence, 129 baris, >16KiB) — snapshot, render key, caret,
  teks, dirty, dan riwayat Undo/Redo **tidak berubah**, pointer cache preview tetap.
- **Plan basi**: 600 kasus di mana preview dibuang setelah plan disiapkan; commit
  wajib menolak dan Source/riwayat tetap utuh. Sumber selalu memuat dua root agar
  neighbour Next benar-benar ada (>20 kasus terverifikasi).
- **History lokal terbatas**: 300 operasi acak (insert Unicode, Enter, Backspace,
  Undo/Redo lokal, pindah kursor) pada draft 3 baris; setiap langkah memenuhi batas
  `MAX_BYTES`16KiB dan total history1MiB/100 record.
- **Determinisme**: seed yang sama menghasilkan urutan hasil identik; seed berbeda
  menghasilkan hasil berbeda, membuktikan harness benar-benar menjelajah kasus.

**910 Cargo** =905sebelumnya +**5Rust** property test. **294strict core** unchanged
subset, **63Python**. Tidak ada tes lama yang diubah, dilonggarkan, atau dihapus.

## Kenapa tanpa suite GUI baru

Tidak ada perilaku produk yang berubah, sehingga gate GUI sebelumnya tetap menjadi
bukti yang sah dan **dijalankan ulang penuh** pada commit ini: 20 suite, 394 checks,
28 badan exercise lama tetap AST-identical `bb37223`. Menambah suite ke-21 untuk kode
yang sama hanya akan menduplikasi oracle tanpa menambah bukti.

## Temuan: batas capture `text_input` (Iced 0.13)

Saat mengevaluasi perluasan shortcut7g ke panel paragraf satu-baris dan sel tabel,
source `iced_widget::text_input` (0.13.1) menunjukkan `KeyPressed` dengan fokus aktif
**menangkap** `Enter` (memicu `on_submit`), `PageUp`/`PageDown`, dan `Escape`
(cabang `_ => {}` lalu `Status::Captured`). Konsekuensinya:

- `Ctrl+Enter` pada panel tersebut **sudah** memicu Apply lewat `on_submit` — perilaku
  yang ada, bukan fitur baru.
- `Ctrl+PageUp`/`Ctrl+PageDown` **tidak bisa** dirutekan lewat subscription selama
  input `text_input` fokus; panel itu tidak dapat memperoleh Apply & Previous/Next
  tanpa mengganti widget-nya menjadi `text_editor` (yang akan mengubah perilaku Enter
  dan seluruh gate GUI lama).
- Karena itu shortcut7g sengaja hanya untuk draft `text_editor`; batasan ini kini
  didokumentasikan di komentar `commands::draft_commit_shortcut` agar tidak
  diklaim sebagai fitur yang hilang.

## Execution status

Local fmt/harnessfmt/294strict core/63Python/AST check PASS.

[CI 39898595716](https://github.com/Indrawan007/Hylite/actions/runs/39898595716),
`c2af096a9d9c39ec8f8e4f94cc968dbd9538ac22`, job120234676767 **SUCCESS**,
2026-10-23 **04:07:10–04:17:50 UTC** / **12:07:10–12:17:50 Makassar**. Format,
strict core subset, Clippy, build, **910Cargo**, seluruh **20suite** (**394GUI**)
dan upload artifact lulus. Artifact `12477769679`,**3.253.627bytes**, SHA256
`11c3fd76f5e200e24e196271113e81a580f6ff842a238ee72e5b60cff9143934`,
expires 2026-10-30T04:17:42Z, unexpired; screenshot tidak direview visual.

Iterasi CI sebelum lulus, semuanya pada tes baru: satu `match` arm bertipe `()`
vs `bool`, dua lint Clippy (`useless_format`, `len_zero`), lalu tiga kegagalan
assertion harness sendiri — caret unchanged, indeks neighbour memakai offset basi,
dan ambang cakupan terlalu tinggi. Tidak ada perubahan produk, pelonggaran lint,
atau penurunan oracle untuk membuatnya lulus.

## Batas

- Property test memakai generator deterministik, **bukan** fuzzing bebas-waktu atau
  bukti formal; korpus tetap terbatas pada bentuk blok yang didukung renderer.
- Bukan bukti performa: jalur tetap O(n) salin+parse.
- Batas desktop (Arch/Hyprland/Wayland, IME, RTL, HiDPI, compositor shortcut) tidak
  berubah dan tetap manual.
