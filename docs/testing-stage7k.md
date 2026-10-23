# Stage 7k — baris Source terautentikasi pada draft sel tabel

Melanjutkan [navigasi paragraf satu baris7j](testing-stage7j.md). Stage7i memberi
provenance baris Source untuk panel paragraf; panel sel tabel masih menyebut baris
tabel (`Row 0 = header · body starts at 1`) tanpa memberi tahu **baris Source** tempat
sel itu hidup. Pada tabel besar (atau beberapa tabel dalam satu dokumen) pengguna
harus mencari sendiri.

Stage7k menutup celah itu tanpa perubahan layout dan tanpa gate GUI baru:

- Header panel sel (baris petunjuk pertama, tinggi tetap20px) sekarang berbunyi
  `Row 0 = header · body starts at 1 · Source line 12 · changes only on Apply`.
- Bila target tidak terautentikasi, panel memakai teks lama
  `Row 0 = header · body starts at 1 · Source changes only on Apply`.

Tidak ada tombol, tinggi baris, atau koordinat yang berubah; tidak ada perubahan
semantik Apply, navigasi sel, history, atau guard.

## Kontrak

- **Baca-saja dan terautentikasi.** `Document::cell_line` lebih dulu memanggil
  `preview_cell_payload` — yaitu otentikasi target sel warm (snapshot saat ini, blok
  tabel yang benar-benar dirender, `rows`/`columns` cocok, dan rentang `payload`
  persis sama dengan `editable_cells[row][column]`). Baru setelah itu ia menghitung
  jumlah `\n` sebelum `payload.start` +1.
- Offset byte diperiksa (`is_char_boundary`, batas teks). Target dingin, forged,
  rentang payload yang digeser, atau blok non-tabel menghasilkan `None` → panel
  memakai fallback lama.
- Nilai provenance **disalin sekali** saat sesi dibuka/dipindah (`Draft::line`), tidak
  pernah dihitung ulang dari byte kandidat. Setelah dokumen berubah (mis. `New`/Open
  mengganti dokumen), panel tetap menampilkan salinan lama selama sesi itu hidup,
  tetapi **Apply tetap menolak** karena target basi — provenance adalah informasi,
  bukan otorisasi.
- Baris petunjuk tetap satu baris dengan tinggi20px, sehingga input, baris
  `Apply`/`Cancel`, dan baris navigasi sel tidak bergeser. Teks baru muat di lebar
  panel820px (≤80 karakter pada 13px).
- Prioritas teks footer tidak berubah: error dulu, lalu `navigation_notice`, lalu
  petunjuk default.

## Implementasi

- `document/state.rs`: `Document::cell_line(&preview_cell::Target) -> Option<usize>`.
- `ui/cell_edit.rs`: `Draft.line: Option<usize>` dan fungsi murni `context_line`
  (format + fallback) dipakai oleh baris petunjuk.
- `app/cell_edit.rs`: `line` diisi saat `open_preview_cell`,
  `move_preview_cell`, dan `apply_preview_cell_adjacent` (memakai target hasil
  transisi, bukan target lama).
- **943 Cargo** = 938 sebelumnya + **5 Rust**: 2 dokumen (`preview_cell_tests.rs`),
  2 controller (`src/app/tests.rs`), 1 UI (`src/ui/cell_edit.rs`).
  **294 strict core** subset unchanged, **65 Python**.

Cakupan tes Rust:

- baris Source benar untuk header dan body tabel pertama serta tabel kedua dalam
  dokumen yang sama (baris1,3,9,11);
- target sel dengan `rows`/`columns`/rentang payload yang dipalsukan, blok non-tabel,
  dan snapshot basi ditolak (`None`);
- `cell_line` tidak mengubah snapshot/render key/caret/dirty/undo/redo maupun pointer
  cache preview, dan baris yang sama masih benar setelah Apply atomik;
- controller: baris tampil saat open (tanpa mutasi dokumen), mengikuti `Move` dan
  `Apply & Next` ke sel berikutnya, dan tetap salinan lama (Apply menolak) setelah
  dokumen diganti;
- UI: format `Source line N` vs fallback, dan affordance `Previous`/`Next` tidak
  berubah oleh provenance.

## GUI

Tidak ada suite ke-22: satu-satunya perubahan yang terlihat adalah teks pada baris
petunjuk bertinggi tetap, sehingga seluruh koordinat tombol tetap. Ke-21 suite
(**419 pemeriksaan**) dijalankan ulang utuh dan lulus, termasuk `table-cells` (14),
`cell-navigation` (16), `cell-apply-previous` (16) dan
`paragraph-navigation-single` (25). 28 badan exercise lama tetap AST-identical
`bb37223`, tanpa perubahan bytes/koordinat/oracle/deadline/refocus.

## Execution status

Local `cargo fmt --all -- --check`, **294 strict core**, **65 Python**,
`git diff --check` PASS.

[CI 39908608710](https://github.com/Indrawan007/Hylite/actions/runs/39908608710),
`c8be824`, job120266166695 **SUCCESS**, 2026-10-23 **05:57:06–06:08:15 UTC** /
**13:57:06–14:08:15 Makassar**. Format, strict core subset, Clippy `-D warnings`,
build, **943 Cargo**, seluruh **21 suite** (**419 pemeriksaan**: 385 clipboard —
289 Source + 96 draft input; 12 title konfirmasi; 22 exit native normal), dan upload
artifact lulus. Artifact `12482570864`, **3.316.700 bytes**, expires
2026-10-30T06:08:08Z; digest SHA256 tidak dapat diunduh dari sandbox ini.

Percobaan pertama suite Cargo gagal pada tes UI baru (asumsi sel `(1,1)` punya
tetangga `Next` padahal itu sel terakhir); ekspektasi diperbaiki ke boundary
`Previous`, bukan gate yang dilonggarkan. Tidak ada byte, koordinat, oracle, atau
deadline yang diubah.

## Manual / limits

- [ ]Apakah teks header yang lebih panjang terpotong pada lebar jendela sempit,
  HiDPI, atau font non-default.
- [ ]Arch/Hyprland/Wayland/portal/IME/RTL/GPU. X11 tiny-skia bukan proof target.

Provenance sel hanya informatif: ia tidak menambah navigasi lintas tabel, tidak
memperluas sel yang boleh diedit, dan tidak mengubah kapan Apply boleh dilakukan.
