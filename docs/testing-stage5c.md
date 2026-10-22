# Stage 5c — cache proyeksi preview per revisi konten

Melanjutkan [Stage 5a](testing-stage5a.md) dan [tabel Stage 5b](testing-stage5b.md).
Ini optimasi jalur model/render, **bukan WYSIWYG**, incremental parser, cache
widget/layout, atau bukti peningkatan latensi/RAM terukur.

## Perubahan

`Document` memiliki satu `OnceCell<Vec<Block>>` privat. `preview_blocks()`
membaca raw `buffer_text()` dan menjalankan parser hanya pada cache miss.
Source-only tetap lazy: load/edit tidak otomatis membuat model preview.

Revisi konten bertindak sebagai masa berlaku cache, bukan nomor checkpoint undo
atau native render key. `advance_revision()` menjadi satu jalur increment revisi
+ pembuangan model lama, dipakai effective edit/transaction dan undo/redo.
New/Open/replace membuat Document baru sehingga cache kosong, termasuk saat teks
sama. Tidak ada multi-entry history model, cross-document cache atau worker async.

| Peristiwa | Perilaku cache |
| --- | --- |
| Insert, paste, Enter, Backspace/Delete efektif | Buang model; parse lazy saat preview berikutnya |
| Transform/format/replace/operasi baris efektif | Sama, lewat transaction/record_edit |
| Undo/redo berhasil | Buang model meskipun kembali ke checkpoint lama |
| New/Open/replace | Cache baru kosong, dokumen lama tidak dipakai kembali |
| Gerak caret, selection, Scroll | Pertahankan model |
| `set_caret`/Find/Go to Line merekonstruksi native buffer tanpa mengubah teks | Render key highlighting berubah, model Markdown tetap |
| Edit no-op/ditolak, undo/redo tanpa target | Pertahankan model |
| Save boundary/acknowledgment/path/dirty-only | Pertahankan model |
| Tema, sidebar/layout, Source/Preview/Split | Pertahankan model; warna/widget dibangun dari tema aktif |

Cache bukan buffer editor kedua yang bisa diedit. Model tetap turunan owned dari
Source. Semua jalur source/save/export/history tetap memakai buffer aslinya.
Getter tidak menaikkan revisi, state ID, buffer generation, dirty atau history.
Tidak ada konfigurasi/dep baru atau perubahan `Cargo.lock`.

### Renderer meminjam model

`render_preview` kini menerima slice model. Paragraph, nested list/quote, tabel,
code dan fallback meminjam data model, bukan mengonsumsi/deep-clone AST setiap
view. Span memakai `Cow::Borrowed` untuk teks cached; code/fallback payload juga
dipinjam. Lifetime Rust memastikan model tidak dibuang ketika widget view masih
meminjamnya; tidak ada unsafe atau RefCell guard yang dikembalikan ke view.

Ini **bukan zero-allocation rendering**: widget tree, span vectors, marker list,
label code dan tooltip tetap dibangun ulang. Text engine Iced dapat menyalin atau
membentuk teks untuk layout sendiri. Tema tidak masuk model sehingga cache yang
sama tetap dapat dirender dengan palet/ukuran/layout berbeda.

## Pengujian

- **10 tes integrasi Document/cache baru**: lazy initialization, repeated reads,
  native edits/transactions, undo/redo/branch, navigation/selection/render-key
  reconstruction, no-op/rejected edits dan redo, save boundary/ack, replace
  same/different text, raw source tanpa LF tambahan, serta tabel 12↔13 kolom
  (grid↔fallback) dan undo. Cache occupancy diperiksa **sebelum getter** setelah
  mutasi; hasil dibandingkan dengan parse penuh yang baru.
- **1 tes span baru** memeriksa `Cow::Borrowed`, pointer dan teksnya terhadap run
  model, bukan klaim text engine internal tidak menyalin.
- **1 tes controller/view baru**: model yang sama dipakai lintas tema/mode/sidebar/
  layout tanpa mengubah snapshot, raw text, render key, dirty atau history.
- **1 tes routing mode/fokus baru** menjaga konteks query Find/Go to Line saat
  Source/Split dan menutupnya pada Preview; task fokus diuji nyata oleh GUI.
- Tes renderer lama diadaptasi untuk lifetime borrowed; semua gate struktur,
  decoration, inert link, tabel dan fallback dipertahankan.
- GUI menambahkan **5 pemeriksaan clipboard**: Source fixture; warm Preview→Split,
  Go to Line posisi Unicode dalam sel Count lalu ketik `9` (`3`→`93`), undo,
  redo, dan kembali ke baseline clean tanpa history tambahan. Seluruh Copy baru
  memakai `refocus=False`; tidak ada klik/windowfocus untuk menutupi kehilangan
  fokus. Screenshot `cache-split.png` dan `cache-split-edited.png` merekam runtime.
- Satu helper Python menjaga fixture dan protokol keyboard/copy tanpa refocus.

GUI membuktikan input/clipboard/history dan runtime tidak crash; **bukan**
pengukuran parse count, stale-pixel oracle atau pembuktian sel preview terlihat
berubah. Invalidation/model freshness dibuktikan oleh tes Document/native.
Tidak menambah parse counter produksi, debug UI, environment flag atau hook GUI.

## Status eksekusi — lulus

[CI run 39719585330](https://github.com/Indrawan007/Hylite/actions/runs/39719585330),
commit `b614f98d2481e3a472bac9ab15faca397d0b26c1`, job `119629706167`:
**success**, Ubuntu 24.04 / Rust stable, 2m43s.

| Gate | Hasil yang diamati |
| --- | --- |
| Format / independent Source core dengan lint ketat | Lulus; 150 core |
| Clippy `--locked --all-targets --all-features -- -D warnings` | Lulus |
| Build native debug semua target/fitur | Lulus |
| Cargo test semua target/fitur | **374 passed**, 0 failed, 0 ignored |
| Python helper | **17 passed**: 3 reporter + 14 runner GUI |
| GUI X11/Xvfb + tiny-skia | **76 pemeriksaan lulus**: 62 clipboard + 12 title + 2 native exit |

150 core, 35 tes parser model dan 13 tes Rust baru Stage 5c **termasuk dalam 374
tes Cargo**, bukan tambahan di luar angka itu. Lokal: core/helper/format/diff
serta 35 tes parser murni dengan source parser versi lockfile juga lulus.

[Artifact screenshot/log/summary](https://github.com/Indrawan007/Hylite/actions/runs/39719585330/artifacts/12400019548)
merekam jendela sebenarnya, mengikuti retensi 7 hari. Tidak ada assertion visual
pixel, benchmark latensi atau pengukuran RSS dalam kelulusan ini.

### Bug fokus yang ditemukan GUI

[Percobaan awal](https://github.com/Indrawan007/Hylite/actions/runs/39719222750)
lulus build/Clippy dan 373 tes Rust, serta edit sel/undo/redo di Split. Namun
Split → Source menghilangkan fokus native editor sehingga Undo fixture tidak
diterima; title tidak kembali clean. Ini bukan bukti cache stale dan bukan
alasan menambahkan klik ke probe.

`SwitchMode` dan `CycleMode` sekarang mengembalikan task `restore_input_focus()`:
Preview no-op; pada mode editable query Go to Line/Find yang masih terbuka
didahulukan, selain itu operasi scoped Source idempoten. Tidak memakai
`focus_next`, caret mutation atau akses private widget. Satu regression routing
headless ditambahkan; probe GUI asli tetap keyboard-only sampai kembali ke
welcome byte-exact dan title clean. Gate lama, batas waktu dan assertions tidak
dilonggarkan. Fokus query lintas mode secara visual tetap perlu desktop target.

## Batas / pekerjaan berikutnya

- Satu model tersimpan tetap berukuran O(source/hasil parser), tanpa byte budget
  cache baru. Cache hangat dapat bertahan saat kembali ke Source; edit berikutnya
  segera membuang model lama. Budget history 200 langkah/8 MiB bukan budget RAM
  cache/model/widget atau proses keseluruhan.
- Perubahan efektif saat Split tetap full parse O(n) di thread UI pada view
  berikutnya. Belum debounce, background parse, incremental parsing, layout
  cache atau virtualisasi. Batas tabel/nesting sebelumnya tetap berlaku.
- Belum benchmark cold/hot parse, frame time, RSS atau file besar; tidak mengklaim
  speedup numerik, FPS, RAM turun atau desktop bebas lag.
- Provenance masih bukan source↔caret/selection mapping, belum editable AST.
- Arch/Hyprland/Wayland, GPU/HiDPI, IME/RTL dan inspeksi visual tetap memerlukan
  desktop target. Portal/save-before-close end-to-end di luar scope cache ini.

Gate strict, lockfile, private Xvfb/XDG, batas waktu dan byte-exact assertions
lama tetap dipertahankan. Tidak ada merge ke `main`.
