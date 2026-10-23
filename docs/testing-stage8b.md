# Stage 8b — workspace folder + file tree (tanpa watcher)

Stage 8a membuka banyak dokumen sekaligus. Stage 8b memberi cara **menemukan**
dokumennya: satu folder kerja di-scan sekali menjadi pohon file di panel kiri,
dan setiap baris adalah tombol nyata.

Yang dikerjakan:

- `hylite <folder>` (argumen CLI, juga dipakai suite GUI) membuka folder itu
  sebagai workspace; `hylite <file.md>` membuka file-nya di buffer tunggal.
  Tanpa argumen, perilaku lama (welcome buffer) tidak berubah.
- `Ctrl+Shift+O` memilih folder lewat dialog native, tombol **Folder** di header
  sidebar melakukan hal yang sama (jalur yang sama, bukan implementasi kedua).
- Tombol **Refresh** di header dan tombol **F5** menjalankan scan ulang.
- Baris folder di-toggle (collapse/expand); baris dokumen membuka file lewat
  jalur `Open` yang sudah ada, termasuk konfirmasi *unsaved changes*.
- `Alt+J` / `Alt+K` memindahkan seleksi keyboard (clamp di kedua ujung), `Alt+Enter`
  mengaktifkan baris terpilih. Modifier harus **Alt persis**, jadi AltGr
  (`Ctrl+Alt`) tidak pernah menyentuh pohon maupun tab.
- Penanda per file: `•` bila tab mana pun yang memegang path itu kotor, warna
  berbeda untuk file yang sedang aktif, `▸`/`▾` untuk folder tertutup/terbuka,
  dan `…` untuk folder yang tidak ditelusuri karena batas.

## Kontrak scan (semua batas ini disengaja)

| Aturan | Nilai | Alasan |
|---|---|---|
| Kedalaman maksimum | `MAX_DEPTH = 8` | folder yang lebih dalam tetap tampil, isinya tidak ditelusuri (`…` + status truncated) |
| Entri maksimum | `MAX_ENTRIES = 512` | satu scan tidak pernah tumbuh tanpa batas; sisanya dihitung `skipped`/truncated |
| Ekstensi dokumen | `md`, `markdown`, `mdx`, `txt` (case-insensitive) | file lain diabaikan diam-diam, bukan error |
| Nama tersembunyi | diabaikan | tidak dihitung sebagai `skipped`, karena ini pilihan desain |
| Symlink | **tidak pernah diikuti**, dihitung `skipped` | symlink bisa keluar folder atau membentuk loop |
| Entry tak terbaca | dihitung `skipped`, scan tetap lanjut | satu folder tanpa izin tidak mematikan seluruh tree |
| Urutan | folder dulu, lalu file; nama case-insensitive, tie-break byte | stabil di filesystem mana pun |

**Tidak ada watcher.** Perubahan di disk hanya masuk setelah `F5`/Refresh. Ini
batasan yang dinyatakan, bukan bug: menonton filesystem butuh crate tambahan dan
punya perilaku sendiri (rename, symlink, limit inotify) yang belum diuji di sini.

Scan berjalan di blocking pool (`tokio::task::spawn_blocking`), tidak pernah di
UI thread. Setiap scan diberi nomor urut (`workspace_seq`), jadi hasil scan folder
lama yang datang terlambat **tidak bisa** menimpa pohon folder yang dipilih
belakangan — ini diuji langsung (`a_late_workspace_scan_can_never_replace_a_newer_one`).

## Integrasi dengan state yang sudah ada

- Buka file dari tree = `PendingAction::OpenAt(path)` → jalur `Open` yang sama:
  operasi file bertoken, snapshot guard, dialog konfirmasi bila buffer kotor.
  `PendingAction` tidak lagi `Copy` (ia membawa `PathBuf`); pembaca yang butuh
  nilai memakai `clone()`/`as_ref()` — tidak ada pembaca yang dilonggarkan.
- `Ctrl+O` tetap membuka **satu file**; folder hanya lewat `Ctrl+Shift+O`.
- Pesan pohon (`OpenWorkspace`, `RefreshWorkspace`, `ToggleWorkspaceDir`,
  `SelectWorkspaceRow`, `ActivateWorkspaceRow`, `OpenTreeFile`) termasuk daftar
  gestur yang **dilaporkan** saat draft/konfirmasi masih terbuka, jadi tidak ada
  klik mati tanpa penjelasan.
- Sidebar mempertahankan header, nama file aktif, dan hint lama; pohon hanya
  mengisi area isi, sehingga lebar sidebar (220px default) dan koordinat editor
  tidak berubah. Semua suite GUI lama dijalankan ulang tanpa perubahan.

## Tes

**Rust (`975` Cargo, +15 dari 960; `306` strict core, +9 dari 297)**

- `src/workspace.rs` (9, ikut strict core karena murni `std`): urutan & filter,
  file-bukan-folder = error, collapse/expand row-for-row, clamp navigasi tanpa
  wrap dan tanpa baris tersembunyi, aktivasi folder vs dokumen, seleksi mengikuti
  path saat rescan, truncation di `MAX_ENTRIES`, batas kedalaman, symlink.
- `src/app/tests.rs` (6): buka dari tree tanpa dialog + snapshot tidak berubah,
  konfirmasi untuk buffer kotor, collapse menghapus baris dari seleksi,
  scan terlambat ditolak, refresh/navigasi melapor saat tidak ada workspace,
  matriks shortcut (Alt persis, AltGr ditolak, `Ctrl+O` tetap file, `Ctrl+Shift+O`
  folder, `F5`).

**Python (`75`, +4)**: probe memastikan seluruh gerak pohon **keyboard-only tanpa
satu klik pun**, urutan copy persis, jumlah key persis, urutan title persis,
fixture UTF-8 persis, dispatch suite eksklusif, dan suite menolak jalan tanpa
folder.

**GUI (`25 suite / 483 pemeriksaan`, +12 dari 471)** — suite ke-25 `workspace`,
boot dengan argumen folder:

1. buffer welcome bersih dan tetap utuh setelah `Alt+J` ×10 (clamp di baris
   terakhir — sekaligus membuktikan navigasi tidak mengetik apa pun);
2. `Alt+Enter` membuka dokumen terpilih; isi clipboard = byte file, title
   `beta.md — Hylite`;
3. collapse folder → `Alt+Enter` tidak membuka apa pun, baris berikutnya adalah
   dokumen berikutnya (`alpha.md`), membuktikan baris anak hilang dari navigasi;
4. expand → baris anak kembali dan bisa dibuka (`nested.md`);
5. file `gamma.md` ditulis ke disk **saat aplikasi berjalan**; hanya setelah `F5`
   file itu bisa dibuka (`gamma.md`) — bukti tidak ada watcher;
6. close native normal (exit 0).

## Execution status

[CI 39941613766](https://github.com/Indrawan007/Hylite/actions/runs/39941613766),
`bd73dd9`, job `120374147972` **SUCCESS**, 23 Oktober 2026 10:47:52–11:00:15 UTC /
**18:47:52–19:00:15 Makassar**: format, strict core, Clippy `-D warnings`, build,
**975 Cargo**, **25 suite / 483 pemeriksaan** (441 clipboard: 315 Source + 119
draft input + 7 tree open; 16 title — 12 konfirmasi + 4 window; 26 exit native),
dan upload artifact lulus.

Artifact `12496547458`, **3.659.862 bytes**, expires 2026-10-30T11:00:12Z, digest
SHA256 dilaporkan API `17c88ad106955cbf2152cab060096461c4ac410d5631f2d8ab10988356c9eb37`
(zip tidak dapat diunduh dari sandbox ini).

Percobaan CI untuk stage ini gagal lima kali sebelum hijau, semuanya karena
kompilasi/ekspektasi tes baru — bukan gate yang dilonggarkan: `Element<'_>`
ambigu pada dua referensi, `PendingAction` tidak boleh `Copy` setelah membawa
`PathBuf` (dan 3 pemakaian di tes lama ikut memakai `clone()`), `Hylite::new`
menjadi dead code lalu dihapus (satu konstruktor: `with_launch_path`), dan dua
ekspektasi tes saya sendiri salah (`text()` selalu menambah LF akhir sesuai
konvensi Iced; seleksi awal ada di baris 0 sehingga setelah collapse justru tetap
di folder).

## Manual / limits

- [ ] Baris pohon dengan mouse: tombol nyata, tapi **tidak** diuji lewat klik —
  suite sengaja keyboard-only agar tidak ada koordinat yang bergeser diam-diam.
- [ ] Penanda kotor per file hanya diuji di level Rust, belum diverifikasi visual.
- [ ] Folder besar (>512 entri) belum dicoba di mesin nyata; perilakunya
  dideklarasikan (truncated + status), bukan diukur.
- Belum ada: watcher, rename/delete/create dari tree, drag, sort pilihan user,
  filter/search di tree, atau membuka folder sebagai project state yang disimpan.
- [ ] Arch/Hyprland/Wayland/portal/IME/GPU. X11 tiny-skia bukan proof target.
