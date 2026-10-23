# Stage 8a — tab multi-buffer: park seluruh `Document`, bukan rebuild

Fase A ([roadmap](roadmap.md)) dimulai di sini. Sampai 7n, Hylite hanya punya **satu**
buffer: `Hylite::document`. Membuka file lain menimpa buffer itu, dan `New` mengosongkan
dokumen yang sedang aktif. Untuk dokumen nyata itu satu-satunya penghalang terbesar
setelah WYSIWYG.

Stage8a menambahkan tab tanpa menyentuh satu pun jalur edit yang sudah ada:

- `Ctrl+T` = tab baru (buffer kosong, langsung aktif), `Ctrl+W` = tutup tab aktif,
  `Alt+1`…`Alt+9` = pilih tab, tombol `＋` dan `›` (cycle) di strip tab.
- Strip tab berada **di dalam baris toolbar yang sudah ada**, sehingga seluruh
  koordinat editor di bawahnya tidak bergeser satu piksel pun.
- Perpindahan tab mempertahankan **teks, caret, seleksi, snapshot, path, dirty flag,
  dan stack Undo/Redo masing-masing buffer** — karena yang diparkir adalah seluruh
  `Document`, bukan hasil salin sebagian.

Yang secara eksplisit **tidak** dikerjakan di 8a: drag-reorder tab, split pane,
session restore, tab per-window, indikator dirty di window title, dan dialog
konfirmasi tutup-tab (tutup tab kotor **ditolak** dengan pesan status, bukan prompt).

## Kontrak

- **Satu `Document` = satu buffer.** `Document` sudah memiliki content native,
  caret/seleksi, history bounded, snapshot/checkpoint, path, dan dirty flag. Karena
  itu `Tab` hanya menyimpan `Option<Document>`: `None` berarti "slot ini adalah tab
  aktif", sebab `Document`-nya hidup di `Hylite::document` agar semua jalur edit lama
  tetap memakai borrow yang sama. Tidak ada tipe buffer kedua, tidak ada salinan text.
- **Pindah tab = dua `mem::replace`, bukan parse ulang.** Tidak ada `Document::new`
  dari teks, tidak ada kehilangan history, dan tidak ada edge history baru di Source
  mana pun (dibuktikan snapshot yang identik sebelum/sesudah).
- **Eksklusif.** Tab tidak bisa dibuat/dipindah/ditutup saat panel draft terbuka,
  saat ada konfirmasi tertunda, atau saat operasi file aktif. Draft memang terikat
  pada target di buffer aktif, jadi guard ini mencegah target basi.
- **Tab terakhir dilindungi** dan **tab kotor tidak pernah dibuang diam-diam**:
  keduanya ditolak dengan pesan status; buffer tetap utuh.
- **Navigasi milik buffer.** Pindah tab me-reset `GoTo` dan hasil pencarian untuk
  dokumen baru (`search.reset_document()` + `refresh_search()`), sama seperti
  `Open`, sehingga tidak ada match/target dari buffer sebelumnya yang bocor.
- **Judul jendela tidak berubah bentuknya** supaya semua oracle title lama
  (`[paragraph draft]`, `•`, `— Hylite`) tetap valid; identitas tab hidup di strip.

## Implementasi

- `src/app/tabs.rs` (baru): `Tab`, `tab_count`, `active_tab`, `tab_document`,
  `tab_title`, `tab_switch_allowed`, `new_tab`, `switch_tab`, `cycle_tab`,
  `close_tab`, `close_active_tab`.
- `src/app.rs`: field `tabs: Vec<tabs::Tab>` + `active_tab: usize`, dispatch pesan
  tab, dan inisialisasi satu tab aktif.
- `src/commands.rs`: `NewTab`, `SwitchTab(usize)`, `CycleTab(bool)`, `CloseTab(usize)`,
  `CloseActiveTab`; shortcut `Ctrl+T`, `Ctrl+W`, dan `Alt+1..9` (Alt persis, jadi
  AltGr/`Ctrl+Alt` tidak pernah memindah buffer). **`Ctrl+Tab` sengaja tidak dipakai**:
  editor Source bisa menangkap Tab bermodifier, sehingga cycling lewat tombol `›`.
- `src/ui/tabs.rs` (baru): strip tinggi 28px lebark 330px di dalam toolbar, label
  `N nama` (14 karakter), `×` untuk tab aktif, `＋` untuk tab baru, `›` untuk cycle.
- `src/ui/toolbar.rs`: strip disisipkan **sebelum** `horizontal_space()`, jadi
  tombol mode di kanan tetap pada posisinya.

**960 Cargo** = 956 sebelumnya + **4 Rust** (`src/app/tests.rs`): park/history/caret,
guard eksklusif (draft + konfirmasi), tutup tab (kotor/terakhir/neighbour), dan
matriks shortcut (Ctrl persis, Alt persis, `Alt+0` ditolak, `Alt+Ctrl+3` ditolak,
accelerator konfirmasi `Alt+S` tetap utuh).

## GUI

Suite ke-24 `tabs` (independen, **tanpa satu pun klik** supaya tidak ada koordinat
toolbar yang bisa bergeser diam-diam):

1. `Ctrl+T` memarkir buffer welcome dan membuat buffer kosong — dibuktikan dengan
   paste satu fixture dan copy yang berisi **hanya** fixture itu;
2. `Alt+1` mengembalikan welcome byte-for-byte, `Alt+2` mengembalikan tab kedua;
3. `Ctrl+Z` di tab kedua tidak menyentuh tab pertama (`Alt+1` masih welcome);
4. `Ctrl+W` menutup tab pertama yang bersih dan jatuh ke tetangganya; `Ctrl+W` pada
   tab terakhir ditolak dan buffer tetap ada;
5. `Ctrl+Z` terakhir mengembalikan tab yang tersisa ke checkpoint kosong (judul
   bersih, oracle lama), lalu close native normal.

Jumlah yang dipertahankan apa adanya: 8 `expect_copy` (semua `refocus: False`),
0 klik, 1 `ctrl+t`, 2 `ctrl+w`, 2 `alt+1`, 1 `alt+2`, 2 `ctrl+z`, 2 `ctrl+a`,
2 `ctrl+v`, 3 `title_is_dirty`, 1 `request_close` + 1 `expect_native_exit`.
28 badan exercise lama tetap AST-identical `bb37223`; 23 gate lama dijalankan ulang
utuh tanpa perubahan bytes/koordinat/oracle/deadline/refocus.

## Execution status

Local `cargo fmt --all -- --check`, **297 strict core**, **71 Python** (2 helper
suite baru), `git diff --check` PASS.

[CI 39936002455](https://github.com/Indrawan007/Hylite/actions/runs/39936002455),
`e6677d7`, job120355571872 **SUCCESS**, 2026-10-23 **10:00:42–10:15:05 UTC** /
**18:00:42–18:15:05 Makassar**. Format, strict core subset, Clippy `-D warnings`,
build, **960 Cargo**, seluruh **24 suite** (**471 pemeriksaan**: 434 clipboard — 315
Source + 119 draft input; 12 title konfirmasi; 25 exit native normal) lulus, dan
upload artifact lulus.

Artifact `12494617096`, **3.386.090 bytes**, expires 2026-10-30T10:15:00Z; digest
SHA256 yang dilaporkan API: `69838e42f990f80ba437f7e644d3d8e7455115969bc23f2eefbc09130113d1bf`
(zip artefak tidak dapat diunduh dari sandbox ini).

Percobaan CI pertama gagal pada **kompilasi/test**, bukan pada gate: (1) helper
`app()` di `src/app/tests.rs` adalah struct literal sehingga field tab baru wajib
diisi; (2) `CycleTab` tidak pernah dikonstruksi → `dead_code` di bawah `-D warnings`
(lalu dijawab dengan tombol `›`, bukan `#[allow]`); (3) test guard mengetik di mode
`Preview` sehingga dokumen uji tidak pernah kotor dan konfirmasi tidak muncul.
Semua perbaikan dilakukan pada kode/fixture, tanpa melonggarkan oracle apa pun.

## Manual / limits

- [ ] Strip tab pada HiDPI, font non-default, atau jendela sangat sempit (label
  dipotong 14 karakter dan container di-clip; belum diuji visual).
- [ ] Perilaku ketika compositor Hyprland sudah memakai `Ctrl+T`/`Ctrl+W`/`Alt+digit`.
- [ ] Belum ada: drag-reorder, indikator tab kotor di window title, dialog
  konfirmasi tutup-tab, session restore, atau split pane.
- [ ] Arch/Hyprland/Wayland/portal/IME/GPU. X11 tiny-skia bukan proof target.
