# Smoke test GUI Source pada X11 virtual

## Hasil yang benar-benar diamati

**Lulus**, [CI run 39711916704](https://github.com/Indrawan007/Hylite/actions/runs/39711916704),
commit `72835cbc2c05472b370677cc46c70d0094d47bc0`, Ubuntu 24.04.
Ada **34 pemeriksaan clipboard berhasil**, termasuk **23 pemeriksaan navigasi/fokus
keyboard-only**. Build, Clippy `-D warnings`, 315 tes Cargo dan 11 tes Python
helper juga lulus pada run yang sama.
[Artifact screenshot/log/summary](https://github.com/Indrawan007/Hylite/actions/runs/39711916704/artifacts/12397800469)
tersedia selama masa retensi. Pemeriksaan GUI tidak dijumlahkan sebagai unit
test Rust; 150 tes portable tetap merupakan subset 315 tes Cargo.

Smoke awal pada `ee73f2b` hanya mempunyai 11 pemeriksaan yang memakai klik untuk
Copy; perluasan ini sengaja tidak memakai klik saat menguji panel/navigasi/undo.

Percobaan startup pertama menemukan `libxkbcommon-x11` yang belum terpasang;
dependensi runtime CI dan petunjuk Arch telah diperbaiki. Runner juga diperbaiki
untuk menghindari write Copy tertunda, memberi jeda antar batch shortcut Iced,
dan membatasi thread serta memberi anggaran capture screenshot terpisah 15 detik.
Smoke awal tidak mengubah logika editor Rust. Perluasan navigasi kemudian
menemukan bug fokus yang memang memerlukan perbaikan aplikasi (lihat di bawah),
bukan klik penyelamat atau pelonggaran ekspektasi teks.

Update lifecycle: [CI run 39713905090](https://github.com/Indrawan007/Hylite/actions/runs/39713905090)
pada `d03197b` juga lulus **320 tes Cargo**, **14 tes Python helper** dan **63
pemeriksaan GUI**. Angka 34 di atas adalah cakupan navigasi awal; suite sekarang
menambahkan New/Cancel/Discard, pemulihan fokus query, dan dua native exit.
Lihat [hasil dan batas lifecycle](testing-lifecycle-x11.md) untuk rincian.

## Cakupan

Runner `scripts/test-gui-x11.sh` membuka **aplikasi Hylite sebenarnya**, bukan test
controller atau widget tiruan, pada X server Xvfb privat dengan renderer CPU
`tiny-skia`. Ia mengirim event mouse/keyboard menggunakan `xdotool` dan membaca
clipboard X11 melalui `xclip`. Tidak ada flag/hook pengujian pada aplikasi Rust.

Pemeriksaan yang dijalankan:

1. Jendela terlihat, isi awal Source sama persis dengan `src/welcome.md`.
2. Paste UTF-8 (`café 世界🙂`), tab sebelum LF dan tab di akhir dokumen.
3. Paste dengan dua LF akhir, termasuk terminal empty line.
4. Undo setiap paste mengembalikan welcome dan title clean; redo memulihkan
   fixture dan title dirty. Reset ke baseline juga diperiksa.
5. Tiga karakter `XYZ` dari event keyboard di akhir dokumen; satu undo
   mengembalikan baseline clean sebagai satu burst ketikan.

Perbandingan clipboard **byte-exact**, tanpa `strip()` atau normalisasi LF.
Sebelum Copy, runner mengganti clipboard dengan marker acak, sehingga membaca
kembali data yang sebelumnya disiapkan untuk Paste tidak dianggap kelulusan.
Transisi title clean → dirty mengakui Paste sudah mengonsumsi clipboard sebelum
marker Copy mengganti clipboard tersebut. Copy dikirim tepat satu kali, lalu
runner hanya mempoll hasilnya sampai ada acknowledgment. Mengulang Copy dalam
poll berisiko meninggalkan write clipboard asinkron yang mencuri ownership
fixture berikutnya, walaupun hasil Copy sebelumnya sudah terlihat benar.
Ada jeda 250 ms antar kombinasi keyboard seperti input manusia: Iced memproses
Select All sebagai message, tetapi Copy membaca seleksi langsung dari widget.
Tanpa jeda frame/update, Copy dapat membaca seleksi klik sebelumnya. Jeda bukan
assertion kelulusan; runner tetap menunggu clipboard baru dan byte-exact match.

## Navigasi dan fokus tanpa mouse

Setelah skenario clipboard awal, fixture terpisah dipasang:

```text
head
café🙂世界
same needle
other needle
```

- Ctrl+G mengambil fokus query dan mengisi `1:1` setelah Ctrl+Home. Query yang
  diedit menjadi `2:6` disalin kembali untuk membuktikan input panel yang aktif.
  Enter lalu ketik `X` tanpa klik: hasil harus `café🙂X世界` pada baris kedua.
  Kolom scalar setelah emoji berbeda dari byte-column; assertion bukan memakai
  resolver/planner produksi sebagai oracle.
- Undo `X` harus tetap menerima keyboard dan mengembalikan fixture. Alamat
  `999:1` ditolak dan fokus query dipertahankan. Escape lalu ketik `Z` tanpa klik
  harus menyisipkannya di caret awal yang tidak diubah oleh alamat invalid.
- Ctrl+F mengambil fokus query, bukan mengedit dokumen. Enter/F3/Shift+F3 diuji
  terpisah untuk match pertama, berikutnya, wrap dan mundur dari state tanpa
  current match. Setelah Escape, `Q` harus masuk tepat sebelum match yang
  diharapkan, bukan di field query atau posisi lain.
- Setiap marker di-undo dan teks dibandingkan dengan fixture; undo fixture
  akhirnya kembali ke welcome clean. Ini memeriksa query/navigasi tidak
  menambahkan entri undo dokumen. Tidak ada pernyataan seluruh state ID/save
  in-flight telah diuji lewat GUI.

`expect_copy(..., refocus=False)` tidak menjalankan mousemove/click/windowfocus;
helper tetap mengganti clipboard dengan marker dan membandingkan byte-exact.
Dua unit test tambahan menjaga mode tanpa refocus serta penggunaan ASCII untuk
`xdotool type`; teks Unicode fixture selalu dimasukkan melalui clipboard.

### Bug yang ditemukan dan diperbaiki

[Run sebelum perbaikan](https://github.com/Indrawan007/Hylite/actions/runs/39711468796)
berhasil pada caret Go to Line, lalu gagal Copy keyboard-only sesudah Undo:
clipboard tetap marker karena Source telah kehilangan fokus. `focus_next()`
bukan setter fokus; ketika editor terakhir sudah fokus, ia justru di-unfocus.
Smoke lama menutup masalah ini dengan mengklik ulang sebelum setiap Copy.

`source_focus.rs` memakai API publik Iced: container bernama yang membungkus satu
Source TextEditor, `operation::scope`, dan `Focusable::focus`. Undo/redo dan
penutupan panel sekarang tidak melepas fokus yang sudah benar. Tiga regresi
Cargo menguji operasi idempoten, isolasi widget lain dan scope yang tidak ada;
smoke GUI memverifikasi task yang benar-benar dieksekusi oleh runtime.

## Cara menjalankan

Build aplikasi terlebih dahulu. Pada Ubuntu, dependensi smoke tambahan:

```sh
sudo apt-get install --no-install-recommends -y \
  xvfb xauth xdotool xclip imagemagick fonts-dejavu-core libxkbcommon-x11-0 python3-xlib
cargo build --locked
./scripts/test-gui-x11.sh
```

Wrapper memeriksa ketersediaan program dan selalu memulai Xvfb baru. Jangan
menjalankan `gui-smoke-x11.py` langsung terhadap DISPLAY desktop pengguna.
`libxkbcommon-x11` dibutuhkan saat runtime X11 walaupun kompilasi berhasil tanpa
library itu; Wayland dan X11 mempunyai jalur runtime berbeda.

Secara default, hasil masuk `target/gui-smoke/`. Direktori harus kosong agar
bukti lama tidak tercampur dengan hasil baru. Untuk percobaan ulang:

```sh
./scripts/test-gui-x11.sh --artifacts "$(mktemp -d)"
```

Path executable bisa diubah lewat `--binary`; gunakan binary yang dibangun dari
checkout yang sama, karena baseline `welcome.md` di-include saat kompilasi.
Runner hanya menulis konfigurasi/cache/data/state/runtime ke direktori sementara,
melepas environment Wayland/Hyprland/D-Bus untuk child, dan tidak mengirim IPC
ke sesi desktop. Clipboard yang disentuh milik Xvfb privat, bukan sesi pengguna.
Proses aplikasi/clipboard owner dihentikan dan direktori sementara dibersihkan
saat selesai atau gagal. Fallback cleanup **bukan** keberhasilan close; suite lifecycle harus mengamati exit alami 0 lebih dulu, lalu menjalankan proses kedua untuk clean close.

## CI dan bukti

Workflow menjalankan smoke sesudah build dan seluruh tes Cargo (kini 530 pada Stage 6e). Output ringkas
tersedia di GitHub Checks; artifact **`gui-x11-smoke`** di halaman run berisi:

- `summary.json`: status, pemeriksaan yang berhasil, dan error bila ada;
- `application.log`: stdout/stderr aplikasi; pesan verbose clipboard owner dibuang agar tidak menenggelamkan error;
- `tabs.png` / `empty-lines.png` / `navigation.png`: tangkapan jendela dengan fixture terpilih (bukan assertion posisi caret visual);
- `confirmation-new.png` / `confirmation-close.png`: tampilan konfirmasi lifecycle;
- `failure.png`: tangkapan best-effort jika window tersedia saat gagal.

`summary.json` juga mencatat `native_exits`; pada suite lifecycle sukses nilainya
2 (dirty close dengan Discard dan clean close), keduanya tanpa force-kill.

Artifact memakai direktori segar di `RUNNER_TEMP` (bukan target cache) dan
retensi 7 hari. Tidak ada dokumen pribadi atau dump environment dalam fixture.
Kegagalan aplikasi, clipboard, screenshot, timeout atau komparasi menggagalkan
step; tidak ada `continue-on-error`. Anggaran global 180 detik, serta batas
per-command/wait dan timeout step CI 5 menit.

Sebelas unit test helper menguji isolasi environment, child exit, timeout,
komparasi byte-exact, pencegahan false-positive clipboard, polling tanpa write
Copy tambahan, mode tanpa refocus, ASCII-only typing, protokol WM_DELETE_WINDOW,
dan exit alami vs signal/timeout. Pada baseline lifecycle, bersama tiga tes reporter CI ada **14 tes
Python**; unit test helper ini **bukan** bukti runtime GUI.

## Batas bukti

Kelulusan smoke hanya mencakup jalur X11/tiny-skia dan skenario di atas.
Screenshot disimpan untuk inspeksi, **tidak** dibandingkan pixel-per-pixel;
keberadaan emoji di clipboard tidak membuktikan glyph dirender dengan benar.
Sebelas pemeriksaan awal menggunakan klik eksplisit. Sebanyak 23 pemeriksaan
navigasi tambahan tidak melakukan refocus mouse; fokus otomatis dan caret
logical pada skenario Go to Line/Find/undo di atas sudah terbukti di X11.
Ini bukan jaminan fokus semua command, kondisi balapan UI, toolbar, Split/Zen,
atau fokus/viewport desktop target Wayland/Hyprland.

Wayland/Hyprland, akselerasi GPU, portal file/D-Bus, Save & Continue dan file chooser,
close saat save in-flight,
IME/RTL, soft-wrap/viewport, mouse selection kompleks dan compositor shortcuts
masih membutuhkan [checklist desktop Tahap 4](testing-stage4.md) beserta
[Tahap 2](testing-stage2.md) dan [Tahap 3](testing-stage3.md). Tes ini tidak
menjadikan Tahap 4 selesai atau aplikasi siap menyimpan dokumen penting.

## Tambahan runner Stage 5a: Preview/Split

**Lulus** pada [CI run 39716266646](https://github.com/Indrawan007/Hylite/actions/runs/39716266646),
commit `ea182b1`: **67 pemeriksaan GUI** (53 clipboard, 12 title, 2 native exit),
347 tes Cargo dan 15 helper Python. Empat pemeriksaan clipboard baru ditambahkan
ke 63 gate lifecycle lama. Satu helper regression menjaga Ctrl+Shift+B (bold),
bukan Ctrl+B (toggle sidebar yang mengubah geometri Split).

Setelah navigasi dan sebelum lifecycle, runner memasang fixture Unicode dengan
heading/inline/list/quote/code/table/HTML/image/math, lalu menjalankan
Source → Preview → Split → Source lewat Ctrl+Shift+M. Ketikan, Enter, bold, dan
Undo saat Preview tidak boleh mengubah Source. Copy baru pada Split dan Source
harus byte-exact, termasuk terminal tab/LF; satu Undo kembali ke welcome clean.
Pada perpindahan mode ini Copy **sengaja mengklik Source**; bukan bukti pemulihan
fokus otomatis. Gate navigasi/lifecycle tanpa klik tetap dipertahankan.

Screenshot tambahan `preview-native.png`, `preview-fallbacks.png` (setelah wheel)
dan `preview-split.png` merekam jendela nyata, tetapi bukan assertion pixel/glyph.
Hasil dan keterbatasan pengujian renderer ada di [Stage 5a](testing-stage5a.md).

## Tambahan runner Stage 5b: tabel native

**Lulus** pada [CI run 39717665832](https://github.com/Indrawan007/Hylite/actions/runs/39717665832),
commit `414c0f0`: **71 pemeriksaan GUI** (57 clipboard, 12 title, 2 native exit),
361 tes Cargo dan 16 helper Python. Empat clipboard check tabel ditambahkan pada
67 gate sebelumnya; navigasi/fokus/lifecycle tidak dikurangi.

Fixture 8 kolom mencakup alignment, inline styles, Unicode, escaped pipe/entity,
missing dan excess body cells, tabel di quote serta tabel 13 kolom fallback.
Wheel horizontal X11 dikirim dan input saat Preview tidak boleh bocor ke Source.
Source/Preview/Split cycle tetap byte-exact sampai terminal tab dan satu Undo
mengembalikan baseline clean. Copy sesudah mode change mengklik Source, bukan
klaim pemulihan fokus otomatis.

`tables-native.png`, `tables-horizontal.png`, `tables-split.png` merekam jendela
sebenarnya. Screenshot bukan assertion posisi scrollbar, alignment/glyph atau
batas sel secara pixel. Detail dan manual checklist ada di [Stage 5b](testing-stage5b.md).

## Tambahan runner Stage 5c: warm cache dan mode fokus

**Lulus** pada [CI run 39719585330](https://github.com/Indrawan007/Hylite/actions/runs/39719585330),
commit `b614f98`: **76 pemeriksaan GUI** (62 clipboard, 12 title, 2 native exit),
374 tes Cargo dan 17 helper Python. Lima clipboard check baru tidak menggantikan
71 gate lama. Semua Copy pada probe baru memakai `refocus=False`.

Source fixture di-copy, Preview dihangatkan lalu Split dibuka. Ctrl+G ke `5:11`
menempatkan caret sebelum angka Count pada baris `| café🙂 | 3 |`; ketik `9`
harus menghasilkan `| café🙂 | 93 |`. Undo/redo dibandingkan byte-exact, lalu
kembali Source dan Undo fixture harus menghasilkan welcome clean tanpa klik.
Percobaan awal menemukan Source kehilangan fokus setelah mode switch; controller
sekarang mengembalikan task konteks query/scoped Source pada tree baru. Tidak
menambah klik, mengubah expected bytes atau memperpanjang timeout untuk lolos.

`cache-split.png` dan `cache-split-edited.png` merekam runtime tetapi tidak
menguji stale pixels/parse count. Kesegaran model/cache occupancy diuji terpisah
pada suite Document. Detail dan batas benchmark/desktop ada di [Stage 5c](testing-stage5c.md).


## Tambahan runner Stage 5d: tombol provenance ke Source

**Lulus** pada [CI run 39728608200](https://github.com/Indrawan007/Hylite/actions/runs/39728608200),
commit `40771f1`: **80 GUI** (66 clipboard, 12 title, 2 exit normal), **396 Cargo**,
**158 core mandiri** (subset Cargo), dan **18 Python**. Empat clipboard check
baru mempertahankan gate sebelumnya:

1. Fixture heading café🙂, body, tabel, code serta terminal tab byte-exact.
2. Source→Preview, klik bar **Show block source** pertama, ketik `X`: harus
   membuka Split dan menyisipkan tepat sebelum `# café🙂` tanpa klik Source.
3. Undo sentinel mengembalikan seluruh fixture termasuk whitespace.
4. Undo berikutnya mengembalikan welcome clean: navigasi bukan edit history.

Semua Copy baru `refocus=False`. Screenshot `reveal-preview.png` dan
`reveal-split.png` adalah evidence runtime, belum ditinjau visual. Klik pada
kontrol preview memang aksi produk yang diuji, bukan workaround fokus editor.
Mode/panel/stale guards dan semua jenis leaf diuji terpisah di Cargo; GUI baru
hanya membuktikan first-heading, bukan klik nested/cell/glyph atau viewport jauh.
Setelah kontrol provenance menambah tinggi layout, probe wheel tabel memakai
`900,300` (bukan `900,200`) agar tetap di area grid. Ini tetap bukan scrollbar
position/pixel assertion. Snapshot tetap frame-local; tidak ada test hook app,
relaksasi expected source, deadline, atau pengujian Wayland/Hyprland baru.
Lihat [kontrak dan batas navigasi Stage 5d](testing-stage5d.md).


## Tambahan runner Stage 5e: klik kanan sel tabel

**Lulus** pada [CI run 39731339080](https://github.com/Indrawan007/Hylite/actions/runs/39731339080),
commit `0a531da`: **84 GUI** (70 clipboard, 12 title, 2 native exit), **411 Cargo**,
**162 core mandiri** (subset Cargo), dan **19 Python**. Empat clipboard check
baru tidak menggantikan gate lama:

1. Fixture header/body/padded row, café🙂, marker strong, code dan terminal tab.
2. Preview: klik kiri Count + ketikan LEAK tetap read-only; klik kanan Count lalu
   X harus mengubah `|café🙂|**3**|` menjadi `|café🙂|X**3**|` dalam Split.
3. Undo mengembalikan source/whitespace byte-exact.
4. Undo berikutnya kembali welcome clean, tanpa history tambahan dari navigasi.

Semua Copy baru `refocus=False`. Dua klik pada preview memang aksi produk yang
diuji, bukan workaround refocus Source. Expected bytes adalah literal, tidak
menggunakan parser produksi. `cell-preview.png` dan `cell-split.png` merekam
jendela, belum ditinjau visual dan bukan pixel oracle. Header/nested/padded/
whitespace/stale/forged guards juga diuji di Cargo tetapi belum semua diklik di
GUI. Tidak ada hook aplikasi, relaksasi timeout/expected bytes atau klaim
Wayland/Hyprland. [Detail dan manual checklist Stage 5e](testing-stage5e.md).


## Tambahan runner Stage 6a: task checkbox sebagai Source edit

**Lulus** pada [CI run 39733641430](https://github.com/Indrawan007/Hylite/actions/runs/39733641430),
commit `c774f2a`: **92 GUI** (78 clipboard, 12 title, 2 exit normal), **439 Cargo**,
**172 core mandiri** (subset Cargo), dan **20 Python**. Delapan clipboard check
baru mempertahankan seluruh gate lama:

1. Fixture canonical unchecked, kapital X lain, café🙂, fenced task literal dan tab.
2. Klik checkbox Preview lalu pilih Split: hanya marker pertama menjadi `[x]`.
3. Satu Undo mengembalikan fixture; Redo memulihkan checked (dua checks).
4. Klik checkbox di panel Split: uncheck dan Source menerima Copy tanpa refocus.
5. Undo kedua toggle kembali checked; Undo pertama toggle kembali fixture; Undo
   paste fixture kembali welcome clean (tiga checks).

Semua Copy `refocus=False`; dua klik produk bukan klik tambahan pada editor.
Expected bytes literal memakai replacement pertama saja, bukan planner/parser
produksi. `task-preview.png`/`task-split.png` adalah evidence runtime, belum
ditinjau visual. Tidak ada assertion pixel atau gate klik checkbox nested/
disabled/query-focus runtime; itu masih checklist manual. Undo tetap Source/
Split, native typing Preview tetap diblokir. Tidak ada hook/timeout/expected
byte yang dilonggarkan. [Kontrak dan batas prototipe Stage 6a](testing-stage6a.md).

## Tambahan runner Stage 6b: history task terbatas di Preview

[CI 39737235985](https://github.com/Indrawan007/Hylite/actions/runs/39737235985),
commit `0df3788`: **101 GUI** (87 clipboard, 12 title, 2 exit normal), **461 Cargo**,
**179 core mandiri** (subset Cargo), **22 Python**. Tambahan 9 clipboard checks:

1. Paste fixture UTF-8/task/kapital X/fenced literal/terminal tab; Copy Source exact.
2. Preview klik checkbox, Ctrl+Z: Split Copy harus fixture semula.
3. Preview Ctrl+Shift+Z: Split Copy harus marker pertama `[x]` saja.
4. Preview Undo lagi: Split Copy harus fixture semula.
5. Preview Ctrl+Y: Split Copy harus marker pertama `[x]` lagi.
6. Preview toolbar Undo: Split Copy harus fixture semula.
7. Preview toolbar Redo: Split Copy harus marker pertama `[x]` lagi.
8. Preview Undo dua kali, ketik LEAK/bold: berhenti sebelum Source paste,
   Split Copy harus fixture semula, tanpa formatting/native typing bocor.
9. Source Undo sekali: welcome exact dan title clean; bukan double dispatch.

Semua Copy baru `refocus=False`, setiap transisi diperiksa lalu mode cycle
kembali Preview. Tiga klik hanya checkbox/Undo/Redo produk, tidak ada klik Source
atau refocus workaround. `task-history-preview.png` dan `task-history-source.png`
merekam runtime tanpa pixel assertion dan belum direview visual. Helper mock
mengunci expected bytes/urutan/jumlah klik/shortcut. GUI budget tetap 180 detik,
exit harus alami 0; tidak ada runtime hook atau gate lama yang diperlonggar.

Percobaan awal `39736591093` gagal startup proses kedua exit 101; reporter lama
memotong diagnostik di Checks API. Chunk error kini 3000 karakter (total tail
maksimal 24.000 karakter, maksimal 8 annotation), dengan satu tes Python baru.
Run `39737235985` lulus seluruh suite, tetapi akar failure startup pertama belum
diketahui dan tidak diklaim diperbaiki oleh reporter. Rincian di
[verifikasi native](verification-native.md) dan [Stage 6b](testing-stage6b.md).
Keyboard compositor, query-task focus runtime, aksesibilitas dan
Wayland/Hyprland tetap perlu desktop target.

## Tambahan runner Stage 6c: level heading root ATX

[CI 39739554830](https://github.com/Indrawan007/Hylite/actions/runs/39739554830),
commit `8e641dd`: **109 GUI** (95 clipboard, 12 title, 2 exit normal), **486 Cargo**,
**187 core mandiri** (subset Cargo), **23 Python**. Tambahan 8 Copy checks:

1. Source fixture exact: `##` + tab + Unicode/raw inline/entity/closing hashes,
   task kapital X lain, fenced heading literal dan terminal tab.
2. Preview H+: hanya satu opening hash ditambah.
3. Preview Ctrl+Z: raw heading semula, tanpa normalize body.
4. Preview Ctrl+Y: satu hash kembali ditambah.
5. Preview H−: raw heading semula lagi, closing hashes/whitespace utuh.
6. Split H+ lalu ketik X: Source menerima keyboard pada marker end, bukan query
   atau display text. X merusak separator heading secara sengaja untuk probe.
7. Source Undo X, Undo heading: kembali fixture; dua transaksi harus terpisah.
8. Undo dua heading Preview lalu Source paste: welcome exact dan clean, tidak
   ada history tambahan dari render/navigasi.

Semua Copy `refocus=False`. Tiga klik hanya kontrol heading produk, bukan Source
atau workaround fokus. Bar provenance lama mempertahankan tinggi; klik source
navigation lama masih diuji pada suite yang sama. `heading-preview.png` dan
`heading-split.png` bukan pixel oracle dan belum direview visual. Mock Python
mengunci 8 expected bytes/urutan/shortcut/klik tanpa parser produk.

Job `119693199908` success; step GUI 07:20:30–07:23:00 UTC pada 22 Oktober 2026.
Budget global 180 detik dan lifecycle exit alami 0 tetap. Tidak ada runtime hook,
retry startup, expected-byte relaxation atau perubahan Source refocus gate.
Setext/nested disabled state, query-focus heading runtime, keyboard accessibility,
HiDPI dan desktop Arch/Hyprland/Wayland belum diverifikasi di jendela nyata.
[Kontrak dan checklist Stage 6c](testing-stage6c.md).

## Tambahan runner Stage 6d: level heading Setext root

[CI 39765116882](https://github.com/Indrawan007/Hylite/actions/runs/39765116882),
commit `cac30e8`: **116 GUI** (102 clipboard, 12 title, 2 exit normal),
**510 Cargo**, **194 core mandiri** (subset Cargo), **24 Python**. Tambahan 7 Copy:

1. Fixture multiline Unicode/raw inline/entity/underline indent dan whitespace,
   task kapital X lain, fenced underline literal dan terminal tab.
2. Preview H+: hanya run underline `=====` menjadi `-----`.
3. Preview Undo: seluruh raw heading semula, bukan normalized/ATX.
4. Preview Redo: run hyphen kembali, panjang/indent/suffix/body utuh.
5. Split H− lalu ketik X: keyboard Source menerima sentinel tepat di akhir run
   equals (sebelum suffix), bukan body/query. Ini sengaja merusak underline.
6. Source Undo X lalu Undo Split Setext: run hyphen; transaksi harus terpisah.
7. Undo Preview Setext lalu paste: welcome exact dan clean, tanpa history tambahan.

Semua Copy `refocus=False`; dua klik hanya kontrol produk, tidak ada Source
click/workaround fokus. `setext-preview.png` dan `setext-split.png` bukan pixel
assertion dan belum direview visual. Mock helper mengunci bytes/urutan/shortcut/
klik tanpa memakai parser produk. Job `119778526781` success pada 22 Oktober
2026; step GUI 10:43:46–10:46:30 UTC. Timeout global tetap 180 detik, exit alami
0, tidak menambah hook/retry atau melonggarkan gate lama. Skenario lama termasuk
ATX, Source frontier, clipboard tab/Unicode dan lifecycle tetap dijalankan.

Query-focus Setext runtime, disabled/HiDPI/accessibility dan desktop
Arch/Hyprland/Wayland belum diverifikasi. [Kontrak Stage 6d](testing-stage6d.md).

## Tambahan runner Stage 6e: batch task dalam suite privat terpisah

[CI 39771530257](https://github.com/Indrawan007/Hylite/actions/runs/39771530257),
commit `3fb4199`: **530 Cargo**, **201 core strict** (subset Cargo), **27 Python**,
**116 regression + 9 task-batch GUI**. Total 125 (110 clipboard, 12 title, 3 exit
alami), bukan menggabungkan ke satu suite dengan timeout yang dinaikkan.

Default `--suite regression` mempertahankan seluruh 116 checks lama dan urutan
exercise/lifecycle. Workflow menambah step `--suite task-batch` dengan wrapper
Xvfb terpisah (app/XDG/env/clipboard temporer baru); alarm 180s dan step 5 menit
masing-masing, job lama 45 menit. Helper mengunci dispatch lama lengkap, suite
unknown ditolak, serta private-display guard kedua mode sebelum start subprocess.

```sh
./scripts/test-gui-x11.sh --artifacts "$(mktemp -d)"
./scripts/test-gui-x11.sh --suite task-batch --artifacts "$(mktemp -d)"
```

Suite batch punya 8 fresh Copy: welcome awal, fixture mixed Unicode/X/quoted/
unsupported tab/fenced literal/terminal tab; Check; Undo Preview; Redo + Check
no-op; Split Clear + sentinel `X` sesudah middle marker byte sebelum `]`; dua
Undo kembali checked (Source typing/Clear terpisah); Undo Check/paste kembali
welcome clean. Setelah initial window/Source setup, semua Copy `refocus=False`;
tiga klik hanya kontrol produk. Satu clean client Close harus exit alami 0;
cleanup paksa bukan PASS. `batch-preview.png`/`batch-split.png` bukan pixel oracle.

Artifact root menyimpan evidence regresi lama; subfolder `task-batch/` menyimpan
summary/log/screenshot baru tanpa overwrite. Summary mencatat suite. Job
`119799991102` success: regresi 11:35:08–11:37:48 UTC dan batch 11:37:48–11:38:01
UTC pada 22 Oktober 2026. Screenshot belum direview visual.

CI awal `39770915857` menemukan label fixed-width wrap yang menggeser toolbar;
font produk diubah dari default 16 ke 14, bukan menggeser koordinat probe atau
menambah Source refocus. Run hijau tetap memeriksa seluruh gate lama. Disabled/
tooltip/query-focus batch runtime dan desktop Arch/Hyprland/Wayland tetap manual.
[Kontrak/cap/auth/history Stage 6e](testing-stage6e.md).

## Tambahan runner Stage 6f: alignment tabel pada suite privat ketiga

`--suite table-alignment` membuka app/Xvfb/XDG/env/clipboard baru, bukan
menambahkan waktu atau mengurangi 116 checks default regression maupun 9
checks task-batch. Ketiga step workflow tetap timeout 5 menit, setiap runner
alarm 180s, job budget 45 menit. Alignment mempunyai 8 exact Copy + 1 client
Close exit alami; empat klik hanya picker dan opsi produk. Semua Copy tanpa
Source refocus. Tidak ada runtime hook/retry.

Fixture mengandung UTF-8, escaped pipe, entity, raw inline, ragged row, indent,
whitespace/dash count dan terminal tab. Probe memilih Column 1 Center di Preview,
Undo/Redo, lalu Column 2 Left di Split, mengetik X tepat sesudah token delimiter,
memisahkan dua Atomic alignments dari native Source typing, dan kembali welcome
clean. Bukti di subfolder `table-alignment/`; screenshot bukan pixel assertion.
Dua Python helper baru mengunci probe dan dispatch eksklusif, guard private
DISPLAY mencakup ketiga suite. [Kontrak/hasil aktual Stage 6f](testing-stage6f.md).

Run pertama 39784221190 lulus 556 Cargo/Clippy/build dan 116+9 GUI lama; suite
baru menemukan collision oracle `replace("--:")` dengan suffix token Center
`:---:` kolom pertama. Actual aplikasi sudah benar pada kolom kedua `:--X`.
Oracle baru mengunci delimiter kedua dengan pipes/spacing dan assertion literal
line independen; bukan mengubah byte expectations gate lama/klik/fokus/deadline.
[CI 39784983010](https://github.com/Indrawan007/Hylite/actions/runs/39784983010)
pada `894227f` **success**: 556 Cargo / 210 core (subset) / 29 Python / **116+9+9
GUI**. Alignment step 13:20:20–13:20:34 UTC, 22 Oktober 2026. Total 134 checks di
**tiga** suite (118 clipboard, 12 title, 4 exit alami); artifact `12428428377`
berukuran 1.668.936 bytes belum expired saat diperiksa, screenshot belum direview
visual. Query-focus runtime/alignment glyph/keyboard menu/HiDPI/accessibility dan
Arch/Hyprland/Wayland/portal/GPU tetap belum diverifikasi.

## Tambahan runner Stage 6g: raw body row dalam suite privat keempat

`--suite table-rows` menambah invokasi private Xvfb/app/XDG/env/clipboard baru,
bukan mengurangi atau memindah 116 regresi, 9 batch atau 9 alignment checks lama.
Keempat suite tetap alarm180s/step5min, budget job45min. Table picker yang sama
kini `Table actions…`; opsi alignment tetap pertama, ukuran dan bar height lama
utuh, sehingga gate alignment dijalankan tanpa perubahan koordinat/bytes/refocus.

Row suite memiliki 8 exact Copy + 1 clean client Close exit alami. Preview Row 1
Duplicate menyalin whole raw LF line (UTF-8/inline/escaped pipe/entity/indent),
Undo/Redo memulihkan source persis. Split Row 2 Delete kemudian sentinel X tanpa
Source refocus membuktikan caret pada awal physical line berikutnya. Dua Source
Undo memisahkan native typing vs Delete; Undo Duplicate/paste kembali welcome
clean. Header/delimiter/ragged row/suffix/terminal tab dipertahankan. Empat klik
hanya kontrol produk, semua Copy `refocus=False`, tanpa runtime hook/retry.

Dua Python helper mengunci literal expected lines independen, shortcut/klik/
no-refocus/exit dan exclusive suite dispatch; private DISPLAY guard diperluas ke
empat suite. Artefak `table-rows/rows-preview.png` / `rows-split.png` bukan pixel
oracle. [Kontrak, hasil aktual dan batas Stage 6g](testing-stage6g.md). Tidak
mengklaim row query-focus runtime, keyboard/menu scroll atau desktop Wayland.

[CI 39787971305](https://github.com/Indrawan007/Hylite/actions/runs/39787971305)
pada `3590d56` **success**: 581 Cargo / 218 core (subset) / 31 Python /
**116+9+9+9 GUI**. Row step 13:41:33–13:41:46 UTC, 22 Oktober 2026. Total143
checks pada empat suite (126 clipboard, 12 title, 5 exit alami), bukan deadline
lebih besar. Artifact `12431131246` berukuran 1.742.544 bytes, belum expired saat
diperiksa; screenshot belum direview visual. Query-focus runtime, menu scroll/
keyboard, glyph/HiDPI/accessibility dan Arch/Hyprland/Wayland/portal/GPU belum
diverifikasi. EOF tanpa LF/non-pipe body row tetap tidak editable lewat row menu.

## Stage 6h: raw cell draft pada suite privat kelima

`--suite table-cells` menambah private Xvfb/app/XDG/env/clipboard baru dan workflow
step5min. Semua 116 regresi + 9 batch + 9 alignment + 9 row lama tetap persis;
15 body exercise functions diperiksa sama secara AST. Picker lama tetap prefix
alignment/row, cell choices ditambahkan di akhir; ukuran/bar height tidak berubah.
Lima runner alarm180s masing-masing dan job45min tetap, tanpa runtime hook/retry/
Source click/refocus workaround atau perubahan oracle gate lama.

Cell suite **14 checks** = 13 exact Copy + 1 natural clean exit. Dua Copy dari
native draft input membuktikan focus/raw initial payload/Unicode clipboard paste;
11 Copy Source membuktikan Cancel/Escape, explicit Apply, Preview Undo/Redo,
unchanged Apply/no extra step, invalid unescaped pipe rejection, Split Apply + X
pada caret payload end, distinct Source Undo transactions dan welcome clean.
Sepuluh klik hanya picker/opsi produk; semua Copy `refocus=False`. Title
`[cell draft]` mengakui panel lifecycle, bukan aplikasi test hook. Dua helper baru
mengunci literal expected line independen/dispatch, private DISPLAY guard kelima
suite berjalan sebelum subprocess. [Kontrak dan batas Stage 6h](testing-stage6h.md).

[CI 39792827763](https://github.com/Indrawan007/Hylite/actions/runs/39792827763),
`3859530`, **success**; 611 Cargo / 226 core (subset) / 33 Python /
**116+9+9+9+14 GUI**. Cells step 14:14:40–14:15:03 UTC, 22 Oktober 2026.
157 checks di lima suite = 139 clipboard (137 Source + 2 draft input) + 12 title +
6 exit alami. Artifact `12432681330` berukuran1.866.592 bytes, belum expired saat
diperiksa, screenshot belum direview visual. `table-cells/cells-preview.png`,
`cells-invalid.png`, `cells-split.png` bukan pixel/glyph oracle. Query/Go-to modal
focus runtime, header/empty/EOF UI, input16KiB/Tab/IME/RTL/accessibility/HiDPI dan
Arch/Hyprland/Wayland/portal/GPU belum diverifikasi. Ini bukan inline WYSIWYG.

## Stage 6i: navigasi draft dalam suite privat keenam

`--suite cell-navigation` memakai Xvfb/app/XDG/env/clipboard sendiri, alarm180s/
step5min dan job45min tetap. Semua16 body exercise lama (12 regression phases,
4 suite independen) sama secara AST dengan6h; oracle bytes, koordinat, no-refocus
dan deadline lama tidak berubah. Tidak ada runtime test hook atau retry.

[CI 39863457232](https://github.com/Indrawan007/Hylite/actions/runs/39863457232),
`983b517`, **success**, **632 Cargo /231 strict core (subset) /35 Python**.
Suite keenam **16 checks** = 15 exact Copy (8 Source + 7 draft) + natural exit0.
Previous/Next melintasi header/body, dirty refusal mempertahankan input, explicit
Unicode Apply & Next memuat neighbor fresh setelah offset berubah. Last-cell
boundary, no-op tanpa extra history, Cancel setelah prior Apply, Preview Undo/
Redo dan Split Source caret/focus tanpa refocus teruji. 13 klik kontrol produk;
semua Copy `refocus=False`. Dua helper baru mengunci literal oracle/action
sequence dan exclusive dispatch; private DISPLAY guard mencakup enam suite.

Total **173 GUI = 116+9+9+9+14+16**: 154 clipboard (145 Source + 9 draft input),
12 title, 7 natural exits. Artifact `12462558973`, 1.975.135 bytes, belum expired
saat diperiksa; screenshot belum direview visual. Evidence `cell-navigation/`
(`cell-navigation-draft.png`, `cell-navigation-split.png`) bukan pixel oracle.
Run22 Oktober UTC /23 Oktober2026 Makassar. Header Apply & Next/EOF UI, query/
Go-to focus runtime, menu keyboard, glyph/HiDPI/accessibility dan desktop Arch/
Hyprland/Wayland/portal/GPU/IME/RTL tetap belum terverifikasi. Tidak ada shortcut
baru atau inline WYSIWYG. [Kontrak dan batas Stage6i](testing-stage6i.md).

## Stage 6j: explicit blank body-row insert, suite privat ketujuh

`--suite table-insert` memakai private Xvfb/app/XDG/env/clipboard sendiri dengan
alarm180s/step5min, job45min tetap. Semua17 exercise bodies lama AST-identical
terhadap8440147; enam gate lama utuh, tidak mengubah bytes/koordinat/refocus dan
tidak ada retry/runtime hook. Dua helper baru mengunci literal row oracle,
four-click/undo sequence, no-refocus/natural exit dan exclusive dispatch. Private
DISPLAY guard mencakup tujuh suite sebelum subprocess.

[CI 39865519334](https://github.com/Indrawan007/Hylite/actions/runs/39865519334),
`d5e009b`, **success**, **649 Cargo /235 strict core (subset) /37 Python**,
22 Oktober22:38:34 UTC /23 Oktober2026 Makassar. New suite **11 checks** =10
exact Source Copy +natural exit0. Preview Insert before/Undo/Redo menjaga raw
UTF-8/markup/escaped pipe/entity/terminal tab; Split Insert after dan native X
membuktikan Source caret/focus di first blank cell. Undo typing/insert/paste
terpisah sampai welcome clean. Opsi baru appended setelah seluruh alignment/
row/cell actions lama; ukurannya tidak mengubah bar height atau prefix index.

Total **184 GUI =116+9+9+9+14+16+11** =164 clipboard (155 Source +9 draft),
12 title,8 natural exits. Artifact `12463529269`,2.049.263 bytes, belum expired
saat diperiksa; screenshot belum direview visual. Evidence `table-insert/`
`insert-preview.png` / `insert-split.png` bukan pixel/glyph oracle. Large menu
scroll/keyboard, ragged/1kolom/12kolom/3spasi UI, Find/Go-to physical focus dan
Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU belum diverifikasi. Header-only
insertion/auto-add row tidak tersedia. [Kontrak dan batas Stage6j](testing-stage6j.md).

## Stage 6k: atomic blank columns, suite privat kedelapan

`--suite table-columns` memakai private Xvfb/app/XDG/env/clipboard sendiri,
alarm180s/step5min, job45min tetap. Semua18 old exercise bodies AST-identical
terhadapddd5afa; tujuh gate lama utuh tanpa perubahan bytes/koordinat/refocus/
deadline dan tanpa retry/runtime hook. Dua helper baru mengunci independent full
Source literals/header-caret/four-click/Undo/no-refocus/exit dan exclusive dispatch;
private DISPLAY guard mencakup semua delapan suite sebelum subprocess.

[CI 39867392194](https://github.com/Indrawan007/Hylite/actions/runs/39867392194),
`9b1e931`, **success**, **672 Cargo /243 strict core (subset) /39 Python**,
22 Oktober22:53:50 UTC /23 Oktober2026 Makassar. New suite **11 checks** =10
exact Source Copy +natural close0. Preview Insert before kolom1 memperbarui blank
header/delimiter/body sebagai satu history edge, raw segments/alignment/tab utuh.
Satu Undo/Redo memulihkan seluruh column; Split Insert after kolom terakhir lalu
native X masuk ke new header cell tanpa Source refocus. Undo typing/column/paste
terpisah sampai welcome clean. Semua old alignment/row/cell/row-insert labels
berada pada prefix sama; new column choices appended dengan width/bar height lama.

Total **195 GUI =116+9+9+9+14+16+11+11** =174 clipboard (165 Source +9 draft),
12 title,9 natural exits. Artifact `12464787718`,2.143.016 bytes, belum expired
saat diperiksa; screenshot belum direview visual. Evidence `table-columns/`
`columns-preview.png` / `columns-split.png` bukan pixel/glyph oracle. Middle/
header-only/1kolom/11→12kolom UI, large menu keyboard/scroll, active Find/Go-to
physical focus, Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU tetap manual.
Tidak ada ragged/column delete/reorder atau inline WYSIWYG. [Kontrak dan batas
Stage6k](testing-stage6k.md).

## Stage 6l: explicit atomic column deletion, suite privat kesembilan

`--suite table-column-delete` memakai private Xvfb/app/XDG/env/clipboard sendiri,
alarm180s/step5min, job45min tetap. Semua19 old exercise bodies AST-identical
terhadap36c137c; delapan gate lama utuh tanpa perubahan bytes/koordinat/refocus/
deadline/retry/runtime hook. Dua helper baru mengunci independent full Source
literals/surviving-header caret/four-click/Undo/no-refocus/exit dan exclusive
dispatch; private DISPLAY guard mencakup sembilan suite sebelum subprocess.

[CI 39871210748](https://github.com/Indrawan007/Hylite/actions/runs/39871210748),
`4926698`, **success**, **694 Cargo /250 strict core (subset) /41 Python**,
22 Oktober23:28:51 UTC /23 Oktober2026 Makassar. New suite **11 checks** =10
exact Source Copy +natural close0. Preview Delete first column memotong header/
delimiter/body sebagai satu history edge, retained Count/right alignment/raw
entity/tab utuh. Satu Undo/Redo memulihkan seluruh column. Split Delete last
column lalu native X masuk ke surviving Name header payload tanpa Source
refocus. Undo typing/delete/paste terpisah sampai welcome clean. Delete choices
appended sesudah seluruh18 label prefix6k; width/font/action-bar geometry lama.

Total **206 GUI =116+9+9+9+14+16+11+11+11** =184 clipboard (175 Source +9 draft),
12 title,10 natural exits. Artifact `12466407895`,2.237.302 bytes, belum expired
saat diperiksa; screenshot belum direview visual. Evidence `table-column-delete/`
`column-delete-preview.png` / `column-delete-split.png` bukan pixel/glyph oracle.
Last-column protection diuji model/Document/controller, bukan menu absence pixel
assertion. Middle/header-only/empty/12kolom UI, large menu scroll/keyboard,
Find/Go-to physical focus dan Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU tetap
manual. Bukan whole-table delete/inline WYSIWYG. [Kontrak dan batas Stage6l](testing-stage6l.md).

## Stage 6m: bounded body-row movement, suite privat kesepuluh

`--suite table-row-move` memakai private Xvfb/app/XDG/env/clipboard sendiri,
alarm180s/step5min, job45min tetap. Semua20 old exercise bodies AST-identical
terhadap69d0ede; sembilan gate lama utuh tanpa perubahan bytes/koordinat/refocus/
deadline/retry/runtime hooks. Dua helper baru mengunci independent full raw lines/
selected-row caret/four-click/Undo/no-refocus/exit serta exclusive dispatch;
private DISPLAY guard mencakup sepuluh suite sebelum subprocess.

[CI 39872957659](https://github.com/Indrawan007/Hylite/actions/runs/39872957659),
`27da9ae`, **success**, **716 Cargo /257 strict core (subset) /43 Python**,
22 Oktober23:43:37 UTC /23 Oktober2026 Makassar. New suite **11 checks** =10
exact Source Copy +natural clean Close0. Preview Row1 Move down menukar whole
raw Unicode/markup/escaped pipe/entity/indent line dengan ragged body neighbor,
header/alignment/other bytes utuh. Satu Undo/Redo memulihkan original/moved order.
Split Row2 Move up lalu native X masuk ke moved row physical start tanpa Source
refocus. Undo typing/move/paste terpisah sampai welcome clean. Move choices
appended setelah seluruh old prefix; boundary actions tidak ditawarkan, tanpa
width/font/action-bar geometry change. Header/bounds/identical no-op protection
diuji unit, bukan menu absence pixel/runtime assertion.

Total **217 GUI =116+9+9+9+14+16+11+11+11+11** =194 clipboard (185 Source
+9 draft),12 title,11 natural exits. Artifact `12467486534`,2.335.915 bytes, belum
expired saat diperiksa; screenshot belum direview visual. Evidence
`table-row-move/row-move-preview.png` / `row-move-split.png` bukan pixel/glyph
oracle. Middle/3+body/1kolom/12kolom/3spasi/trailing-tab UI, no-op/query/Go-to
physical focus, large menu keyboard/scroll dan Arch/Hyprland/Wayland/portal/IME/
RTL/HiDPI/GPU tetap manual. Bukan header/column move/drag atau inline WYSIWYG.
[Kontrak dan batas Stage6m](testing-stage6m.md).

## Stage 6n: bounded column movement with alignment, suite privat kesebelas

`--suite table-column-move` memakai private Xvfb/app/XDG/env/clipboard sendiri,
alarm180s/step5min, job45min tetap. Semua21 old exercise bodies AST-identical
terhadap8fbc789; sepuluh gate lama utuh tanpa perubahan bytes/koordinat/refocus/
deadline/retry/runtime hooks. Dua helper baru mengunci independent full raw
columns/alignment/selected-header caret/four-click/Undo/no-refocus/exit serta
exclusive dispatch; private DISPLAY guard mencakup sebelas suite sebelum subprocess.

[CI 39874500418](https://github.com/Indrawan007/Hylite/actions/runs/39874500418),
`46c89c6`, **success**, **738 Cargo /264 strict core (subset) /45 Python**,
22 Oktober23:59:38 UTC /23 Oktober2026 Makassar. New suite **11 checks** =10
exact Source Copy +natural clean Close0. Preview Column1 Move right menukar raw
header/delimiter/body dengan old alignment mengikuti kolom asal, raw Unicode/
markup/escaped pipe/entity/spacing/tab/prefix/suffix utuh. Satu Undo/Redo memulihkan
original/moved columns. Split Column2 Move left lalu native X masuk ke selected
moved Count header payload tanpa Source refocus. Undo typing/move/paste terpisah
sampai welcome clean. Move choices append setelah old prefix termasuk row moves;
no-wrap/single-column/identical guard diuji unit, bukan pixel/menu absence oracle.

Total **228 GUI =116+9+9+9+14+16+11+11+11+11+11** =204 clipboard (195 Source
+9 draft),12 title,12 natural exits. Artifact `12468122247`,2.433.374 bytes, belum
expired saat diperiksa; screenshot belum direview visual. Evidence
`table-column-move/column-move-preview.png` / `column-move-split.png` bukan pixel/
glyph oracle. Middle/3+kolom/header-only/empty/12kolom/3spasi/trailing-tab UI,
no-op/query/Go-to physical focus, large menu keyboard/scroll dan Arch/Hyprland/
Wayland/portal/IME/RTL/HiDPI/GPU tetap manual. Bukan drag/bulk/resize atau inline
WYSIWYG. [Kontrak dan batas Stage6n](testing-stage6n.md).

## Stage 6o: raw column duplication with alignment, suite privat kedua belas

`--suite table-column-duplicate` punya private Xvfb/app/XDG/env/clipboard sendiri,
alarm180s/step5min dan job45min tetap. Semua22 old exercise bodies AST-identical
terhadap4e96cd2; sebelas gate lama utuh tanpa perubahan bytes/koordinat/refocus/
deadline/retry/runtime hooks. Dua helper baru mengunci independent full raw
column copy/alignment/new-header caret/four-click/Undo/no-refocus/exit dan exclusive
dispatch; private DISPLAY guard mencakup12suite sebelum subprocess.

[CI 39876692641](https://github.com/Indrawan007/Hylite/actions/runs/39876692641),
`9465436`, **success**, **759 Cargo /270 strict core (subset) /47 Python**,
23 Oktober2026 00:19:28 UTC /08:19:28 Makassar. New suite **11 checks** =10 exact
Source Copy +natural clean client Close0. Preview Column1 Duplicate menyalin
header/delimiter/body kolom Name, raw alignment/dash/Unicode/markup/escaped pipe/
entity/spacing/indent/tab/prefix/suffix utuh. One-step Preview Undo/Redo. Split
Column2 Duplicate lalu native X masuk ke copied third Count header, bukan original,
tanpa Source refocus. Undo typing/duplicate/paste terpisah sampai welcome clean.
Empat klik picker/opsi produk; duplicate choices append setelah entire prefix6n,
termasuk row/column moves. Single/empty/header-only/growth guard diuji unit,
bukan pixel/menu absence proof.

Total **239 GUI =116+9+9+9+14+16+11+11+11+11+11+11** =214 clipboard (205 Source
+9 draft),12 title,13 natural exits. Artifact `12468243673`,2.532.220 bytes, belum
expired saat diperiksa; screenshot belum direview visual. Evidence
`table-column-duplicate/column-duplicate-preview.png` / `column-duplicate-split.png`
bukan pixel/glyph oracle. Initial39876046796 gagal satu new controller fixture
(2×2 helper vs128×3), GUI skipped; helper test-only diperbaiki9465436, all12suite
CI ulang success. Tidak mengubah production/refocus/oracle/deadline demi tes.

Middle/3+kolom/single/empty/header-only/11→12kolom/3spasi/trailing-tab UI, physical
Find/Go-to focus, large menu keyboard/scroll dan Arch/Hyprland/Wayland/portal/IME/
RTL/HiDPI/GPU tetap manual. Bukan drag/bulk/resize/implicit Apply atau general
inline WYSIWYG. [Kontrak, bukti dan batas Stage6o](testing-stage6o.md).

## Stage 6p: Apply & Previous raw cell drafts, suite privat ketiga belas

`--suite cell-apply-previous` memakai private app/Xvfb/XDG/env/clipboard sendiri,
alarm180s/step5min/job45min tetap. Semua23 old exercise bodies AST-identical
terhadap803ff5c. Dua belas gate lama, menu dan koordinat input/Apply/Cancel/tiga
navigation button lama tidak diubah. Fourth button160×32 di tail navigation row,
tanpa shortcut. Dua Python probe baru mengunci independent exact raw draft/Source/
alignment/new selected target/caret/click/Undo/no-refocus/close dan exclusive
dispatch; private DISPLAY guard13suite sebelum subprocess.

[CI 39878237851](https://github.com/Indrawan007/Hylite/actions/runs/39878237851),
`6caedff`, **success**, **775 Cargo /270 strict core (subset) /49 Python**,
23 Oktober2026 00:36:51 UTC /08:36:51 Makassar. New suite **16 checks** =15 exact
Copy (9 Source +6 draft) +natural clean Close0. Last raw body draft exact, invalid
input copy sebelum disabled click, title tetap draft; Cancel+Source membuktikan
no partial Apply. Reopen via picker, valid Unicode Apply Previous membuka preceding
raw body payload dengan focus. Unchanged Apply Previous body→header Count→Name
tanpa history. Modified first-header draft sebelum disabled boundary, title tetap
first; Cancel Source membuktikan tidak commit first draft. Ini tidak mengklaim
input focus setelah disabled click: Iced dapat melepasnya, tidak dipulihkan lewat
klik workaround. Cancel/reopen adalah workflow produk eksplisit, bukan retry.

Preview one-step Undo/Redo memulihkan exact Source. Split Apply Previous, Cancel,
native X tanpa Source refocus masuk pada applied payload end (`finalX`), bukan
previous payload. Separate typing/Apply/paste Undo sampai welcome clean, natural
Close0. Source alignment/indent/Unicode/markup/escaped pipe/entity/spacing/tab/suffix
utuh. No runtime hooks/oracle weakening/refocus/deadline changes untuk old gates.
Total **255 GUI =116+9+9+9+14+16+11+11+11+11+11+11+16** =229 clipboard (214 Source
+15 draft),12 title,14 natural exits. Artifact `12469403519`,2.622.926 bytes,
belum expired saat diperiksa; screenshot belum direview visual. Evidence
`cell-apply-previous/cell-apply-previous-draft.png` / `cell-apply-previous-split.png`
bukan pixel/glyph oracle.

Header Apply Previous/EOF/empty/single/caps UI, physical Find/Go-to focus, narrow
window/accessibility/keyboard traversal dan Arch/Hyprland/Wayland/portal/IME/RTL/
HiDPI/GPU tetap manual. Bukan inline WYSIWYG/implicit Apply/session Undo atau
synchronized scrolling. [Kontrak dan batas Stage6p](testing-stage6p.md).

## Stage 7a: single-line root paragraph drafts, suite privat keempat belas

`--suite paragraph-draft` memakai private app/Xvfb/XDG/env/clipboard, alarm180s/
step5min sendiri, job45min tetap. Semua24 old exercise bodies AST-identical terhadap
7c3f84d. Seluruh13 old gates utuh tanpa perubahan bytes/koordinat/deadline/refocus/
oracle. New Edit paragraph136×24 append existing eligible paragraph provenance
bar; Show block source Fill shrinks, bar height/spacing dan heading/table controls
unchanged. Ini bukan klaim semua paragraph source hit regions lama identik.
Dua Python probe baru mengunci independent literal draft/Source/structural refusal/
caret/click/Undo/no-refocus/close and exclusive dispatch; private DISPLAY guard
14suite sebelum subprocess.

[CI 39879922775](https://github.com/Indrawan007/Hylite/actions/runs/39879922775),
`56865a7`, **success**, **801 Cargo /277 strict core (subset) /51 Python**,
23 Oktober2026 00:53:15 UTC /08:53:15 Makassar. New suite **15 checks** =14 exact
Copy (11 Source +3 draft) +natural clean client Close0. Root paragraph raw Unicode/
markup/entity/escaped pipe/trailing spaces copied with native draft focus;
Cancel exact Source/no history. Unicode paste remains staging, explicit Apply
preserves LF/heading/neighbor/suffix/terminal tab. One-step Preview Undo/Redo.
Split unchanged Apply closes no extra edge; Enter `# heading` parser refusal keeps
exact raw draft/title, Cancel preserves Source. Split final Apply +native X at
payload end (`finalX`) without Source refocus. Native typing/Apply/paste history
separate to welcome clean, natural Close0.

Lima klik hanya product Edit paragraph controls. No runtime hooks/retry/refocus
workaround/oracle weakening. Total **270 GUI =116+9+9+9+14+16+11+11+11+11+11+11+
16+15** =243 clipboard (225 Source +18 draft),12 title,15 natural exits. Artifact
`12470087487`,2.811.471 bytes, belum expired saat diperiksa; screenshot belum
direview visual. Evidence `paragraph-draft/paragraph-preview.png`,
`paragraph-invalid.png`, `paragraph-split.png` bukan pixel/glyph oracle.

Multiple root paragraphs/prefix/header neighbors/trailing spaces/16KiB UI,
structural refusal variants selain heading, physical query/Go-to focus, narrow
window/accessibility/keyboard traversal, other Source-bar hit regions dan target
Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU tetap manual. Belum multiline/EOF
paragraph editing atau inline WYSIWYG/glyph mapping/scroll sync.
[Kontrak, bukti dan batas Stage7a](testing-stage7a.md).

## Stage 7b: native multiline root paragraph draft, suite privat kelima belas

`--suite paragraph-multiline` private app/Xvfb/XDG/env/clipboard baru,180s runner/
5min separate step/job45min unchanged. All25old exercise bodies AST-identical
`9ad722d`,14existing gates preserved: no old bytes/coordinate/refocus/oracle/
deadline change. Private DISPLAY validation now15suites; no retry/runtime hooks.

**Lulus** [CI 39881731180](https://github.com/Indrawan007/Hylite/actions/runs/39881731180),
`5d9cca1`,watch exit0,job120180464018,23 Oktober2026 01:03:04–01:11:19 UTC /
09:03:04–09:11:19 Makassar. New suite **16checks** =15exact Copy (11Source+4draft)
+natural clean Close0. Raw Unicode/hard breaks/native draft focus; native Enter
newline staging/Cancel no Source change;3-line clipboard explicit Apply keeps
final LF/neighbors, atomic Preview Undo/Redo; Split unchanged no edge; parser
split refusal retains exact draft/focus without refocus; Cancel no partial Source;
Split final Apply then X at last-line end, typing/Apply/paste Undo to welcome clean.
Nine clicks only product Open/Apply controls, no refocus workaround.

**286 GUI =116+9+9+9+14+16+11+11+11+11+11+11+16+15+16**,258clipboard
(236Source+22draft),12title,16natural exits. Native829Cargo; local282strict-core
subset and53Python, corresponding CI steps pass. Artifact `12470923520`,
2.913.840bytes,unexpired when checked. `paragraph-multiline/multiline-preview.png`,
`multiline-invalid.png`, `multiline-split.png` not visually reviewed/pixel oracles.
Raw check annotations confirm all suite counts; full archived logs unavailable
at blocked Actions results host, not claimed full-log inspection.

Separate native draft Source authority,2–128nonempty/unindented lines/16KiB/final
LF; Enter newline, explicit Apply button, full complete-root preflight/no block
conversion/split.7a single-line unchanged. Root multiline provenance tail button
shrinks Source Fill, not all previous hit regions identical. Capacity/other
structural errors/other hit regions/physical Find-Go-to focus/accessibility/keyboard/
narrow-window and Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU remain manual.
No free inline WYSIWYG/glyph mapping/scroll sync/performance proof.
[Contract/evidence/limits7b](testing-stage7b.md).

## Stage 7c: multiline draft-local Undo/Redo, suite privat keenam belas

`--suite paragraph-draft-history` private app/Xvfb/XDG/env/clipboard,180s runner,
5min own step/45min job unchanged. All26old exercise bodies AST-identical91386ee;
15old gates preserved, no old byte/coordinate/refocus/oracle/deadline change.
Private DISPLAY guard now16suites. No retry/runtime hooks.

**Lulus** [CI 39884654030](https://github.com/Indrawan007/Hylite/actions/runs/39884654030),
`1253655`,watch exit0,job120189875625,23 Oktober2026 01:33:21–01:42:33 UTC /
09:33:21–09:42:33 Makassar. New **24checks** =23Copy(8Source+15draft)+natural Close0.
Focused empty Ctrl+Z/native Unicode3line paste as one step, Ctrl+Y/Shift+Z; invalid
blank/final tab Undo valid/Redo invalid exact; local branch empty redo; Undo draft
button Copy proves input focus without refocus. Cancel Source untouched/reopen no
session redo; Split local Undo/Redo→Apply/native X at payload end, Source typing/
Apply Undo separate, Preview Redo only commit, undo to welcome clean/natural Close0.
Four clicks only product Open/Undo draft/Apply. Source unchanged before Apply;
local history not extra Source edges. No-op/caps/stale/save guards also unit-tested.

**310GUI =116+9+9+9+14+16+11+11+11+11+11+11+16+15+16+24**,
281clipboard(244Source+37draft),12titles,17natural exits. Native852Cargo;
local294strict core subset,55Python, corresponding CI steps pass. Artifact
`12472357915`,3.003.957bytes,unexpired; screenshots
`paragraph-draft-history/draft-history-preview.png`, `draft-history-source.png`
not visually reviewed/pixel oracles; counts via check annotations, not full archive.

Initial39883632691/52e77a0 failed **new probe mode sequence only**:852Rust/all15old
gates and21new Copies pass, Copy attempted in Preview. Test-only1253655 uses
Split→Source→Preview before Redo, then Split Copy; Python asserts5mode cycles and
final key sequence. Expected bytes/coordinates/deadline/refocus untouched; not an
application-code fix or retry. Complete fresh39884654030 passes all16suites.

100retained records/1MiB raw text,16KiB per record, one effective edit one step,
no typing coalescing or selection-anchor/scroll restore. Native cap UI/other edit
variants/disabled-button blur/narrow-window/traversal/accessibility/physical query
focus, Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU still manual. X11 tiny-skia
is not target compositor proof; full snapshots/rebuild O(n), not speedup.
[Contract/evidence/limits7c](testing-stage7c.md).

## Stage 7d: unchanged multiline Previous/Next, suite privat ketujuh belas

`--suite paragraph-navigation` private app/Xvfb/XDG/env/clipboard,180s runner,
5min own step/45min job unchanged. All27old exercise bodies AST-identical a616dd2;
16old gates preserved, no old byte/coordinate/refocus/oracle/deadline change.
Private DISPLAY guard now17suites. No retry/runtime hooks.

**Lulus** [CI 39886493920](https://github.com/Indrawan007/Hylite/actions/runs/39886493920),
`13769ac`,watch exit0,job120195796902,23 Oktober2026 01:53:10–02:02:24 UTC /
09:53:10–10:02:24 Makassar. First feature CI passes. New **19checks**
=18Copy(8Source+10draft)+natural Close0. First root Unicode/hard break native focus;
dirty Next refusal/raw Copy then local Undo→Next skips heading/single/nested/code;
exact second/native focus and previous session redo discarded; Previous first.
Disabled first/last boundaries use Copy-before-click→Cancel preserving Source,
not refocus clicks. Split second changed/dirty Previous refusal leaves Apply
available; explicit Apply then native X at final-line end. Source typing/Apply
Undo separate, first/skipped/suffix bytes untouched, final Undo welcome clean.
12clicks only product Open/Previous/Next/Apply. Selection retention is controller
unit proof, native Copy doesn't assert selection anchor/direction/pixel layout.

**329GUI =116+9+9+9+14+16+11+11+11+11+11+11+16+15+16+24+19**,
299clipboard(252Source+47draft),12titles,18natural exits. Native869Cargo;
local294strict core unchanged subset,57Python, corresponding CI steps pass.
Artifact `12473142607`,3.106.610bytes,unexpired; screenshots
`paragraph-navigation/paragraph-navigation-preview.png`, `paragraph-navigation-dirty.png`
not visually reviewed/pixel oracles. Native counts via check annotations, not full archive.

Previous/Next only unchanged/error-free eligible LF-root2–128line/16KiB, no
wrap/rebase/implicit Apply/creation. Success opens new session/caret0/history empty,
Source unchanged; refusal retains draft/history/focus. New160×32row below old
Apply/Cancel/Undo/Redo, footer44px lower; old input/button/root coordinates retained.
No Apply & Next/Previous/cross-kind/glyph mapping/scroll sync/performance proof.
Capacity/offscreen/more unsupported cases/selection direction/traversal/disabled
blur/narrow-window/accessibility/physical query focus/Arch/Hyprland/Wayland/portal/
IME/RTL/HiDPI/GPU remain manual. Full root walk/raw copies O(n), not speedup.
[Contract/evidence/limits7d](testing-stage7d.md).

## Stage 7e: Apply & Next multiline, suite privat kedelapan belas

`--suite paragraph-apply-next` memakai private app/Xvfb/XDG/env/clipboard, alarm
180s, step5menit sendiri dan job45menit tetap. Seluruh **28old exercise bodies
AST-identical `bb37223`**; 17gate lama dipertahankan tanpa perubahan bytes/
koordinat/oracle/deadline/refocus. Private DISPLAY guard kini18suite. No retry/
runtime hook. Apply Next memakai satu buffer Source baru + record atomic, sesi next
native baru, dan dua fase preflight sebelum mutasi.

**Lulus** [CI 39888436003](https://github.com/Indrawan007/Hylite/actions/runs/39888436003),
`8ba6602`, job120202070235, 23 Oktober2026 02:13:55–02:25:14 UTC /
10:13:55–10:25:14 Makassar. Native format/Clippy/build/**886Cargo** dan 18suite
**352GUI**. Run pertama `64cc13f` berhenti di `unused_mut` tes baru sebelum build/
GUI; perbaikan test-only `8ba6602` tanpa melonggarkan lint, tanpa perubahan produk/
bytes/oracle/deadline.

**352GUI =116+9+9+9+14+16+11+11+11+11+11+11+16+15+16+24+19+23**,
**321clipboard =263Source+58draft**,12titles,19natural exits. Suite baru **23checks**
=22Copy(11Source+11draft)+Close0: raw first focused; tiga baris Unicode staged lalu
Apply & Next mengganti hanya first dan membuka second exact dengan native focus/
offsets baru; local Ctrl+Z kosong; previous grown first; unchanged Next tanpa Source
edge; last changed/disabled boundary→Cancel; satu Preview Undo/Redo; Split
`one\n# heading` refusal tanpa partial; Cancel lalu native X membuktikan caret di
akhir payload first yang diterapkan (bukan second); Undo Source bertahap dan welcome
clean. Sembilan klik hanya product Open/Previous/Apply Next (termasuk disabled last).

Artifact `12474580471`,3.158.202bytes,SHA256
`4d02ac3bb9d5e751d93f0f448488d2ccca9e8a348461f6a9af540a8d8bc3f214`,
unexpired; screenshot `paragraph-apply-next/paragraph-apply-next-refused.png` dan
`paragraph-apply-next-split.png` tidak direview visual/bukan pixel oracle. Native
count dari check annotations, bukan full archive.

Batas: native Copy membuktikan raw value/focus, bukan selection anchor/direction/
glyph mapping. Capacity/offscreen/128line/16KiB manual, disabled blur/keyboard/
narrow window/accessibility/physical query focus/scroll, serta Arch/Hyprland/Wayland/
portal/IME/RTL/HiDPI/GPU/visual review tetap manual. Full candidate copy/parse dan
dua native buffer O(n) +memori ekstra, bukan speedup. [Kontrak7e](testing-stage7e.md).

## Stage 7f: Apply & Previous multiline, suite privat kesembilan belas

`--suite paragraph-apply-previous` private app/Xvfb/XDG/env/clipboard, alarm180s,
step5menit sendiri, job45menit tetap. **28old exercise bodies AST-identical
`bb37223`**; 18gate lama dipertahankan tanpa perubahan bytes/koordinat/oracle/
deadline/refocus. Private DISPLAY guard kini19suite, tanpa retry/runtime hook.

**Lulus** [CI 39892699395](https://github.com/Indrawan007/Hylite/actions/runs/39892699395),
`0353f5e`, job120215906995, 23 Oktober2026 03:00:56–03:11:10 UTC /
11:00:56–11:11:10 Makassar. Native format/Clippy/build/**903Cargo** dan 19suite
**378GUI**. Iterasi sebelumnya (semua test-only setelah produk `2d33fea`): lint
`unused_mut`, empat+ satu assertion tes baru, suite GUI berhenti setelah10checks
karena Copy setelah klik disabled, dan satu assertion judul dirty; tidak ada
pelonggaran gate.

**378GUI =116+9+9+9+14+16+11+11+11+11+11+11+16+15+16+24+19+23+26**,
**346clipboard =273Source+73draft**,12titles,20natural exits. Suite baru
**26checks** =25Copy(10Source+15draft)+Close0: tiga root raw Unicode dengan heading/
quote/fence/list; dua Next unchanged ke root ketiga; edit tiga baris lalu Apply &
Previous mengganti hanya root ketiga dan membuka middle exact dengan focus/offsets
baru; local Ctrl+Z kosong; unchanged Apply & Previous ke first tanpa Source edge;
draft changed lalu klik boundary pertama disabled (Copy dilakukan sebelum klik karena
disabled blur), Escape+Cancel menjaga Source; satu Source Undo/Redo; Split structural
`one\n# heading` refusal lalu Cancel; Split final middle→Apply & Previous membuka
first; Cancel lalu X membuktikan caret Source di akhir payload middle yang diterapkan;
Undo bertahap kembali ke fixture lalu welcome clean; Close0.12klik hanya product
Open/Next/Apply & Previous termasuk satu disabled boundary.

Artifact `12475384207`,3.279.313bytes,SHA256
`92636145659d325378e7c3073665a601d74c93bb1dcfa1bc6b1f95f4574b06a2`,unexpired;
screenshot `paragraph-apply-previous-refused.png`/`-split.png` tidak direview
visual dan bukan pixel oracle. Native count dari check annotations.

Batas: Copy native membuktikan raw value/focus, bukan selection anchor/direction/
glyph mapping; disabled blur tidak diklaim direfocus. Capacity/offscreen/128baris/
16KiB manual, keyboard/narrow window/accessibility/physical query focus/scroll, serta
Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU/visual review tetap manual. Copy full
Source/parse/dua native buffer O(n) +memori ekstra, bukan speedup.
[Kontrak7f](testing-stage7f.md).

## Stage 7g: shortcut commit draft multiline, suite privat kedua puluh

`--suite paragraph-draft-shortcuts` private app/Xvfb/XDG/env/clipboard, alarm180s,
step5menit sendiri, job45menit tetap. **28old exercise bodies AST-identical
`bb37223`**; 19gate lama dipertahankan tanpa perubahan bytes/koordinat/oracle/
deadline/refocus, tanpa retry/runtime hook. Private DISPLAY guard kini20suite.

**Lulus** [CI 39895925701](https://github.com/Indrawan007/Hylite/actions/runs/39895925701),
`0539b02`, job120226246206, 23 Oktober2026 03:36:36–03:47:19 UTC /
11:36:36–11:47:19 Makassar. Native format/Clippy/build/**905Cargo** dan 20suite
**394GUI**. Tiga iterasi sebelumnya test-only (Message bukan PartialEq,
`clone_on_copy`, dan fokus Source setelah Ctrl+Enter dipulihkan lewat mode switch
seperti suite7b); tidak ada pelonggaran gate.

**394GUI =116+9+9+9+14+16+11+11+11+11+11+11+16+15+16+24+19+23+26+16**,
**361clipboard =278Source+83draft**,12titles,21natural exits. Suite baru
**16checks** =15Copy(5Source+10draft)+Close0: **Ctrl+Enter** apply+close satu edge
atomik, **Ctrl+PageDown/Ctrl+PageUp** Apply & Next/Previous melalui guard yang sama
dengan tombol, boundary pertama/terakhir menolak tanpa wrap maupun partial Apply,
draft changed dipertahankan, Escape tak menyentuh Source, dua Undo bertahap kembali
ke fixture lalu welcome clean, Close0. Hanya2klik produk Open; tidak ada
Copy setelah klik disabled.

Artifact `12476433906`,3.253.935bytes,SHA256
`aa7def659290d7b336bc7b0860bbea4db9396b1968e4349391789539d294a138`,unexpired;
suite ini tidak membuat screenshot. Native count dari check annotations.

Batas: shortcut nyata di Hyprland bisa diambil compositor; IME/keyboard non-US dan
aksesibilitas keyboard-only manual. Ctrl+Alt/Logo/Shift sengaja ditolak di unit
test, tetapi perilaku compositor nyata belum diuji. Arch/Hyprland/Wayland/portal/
RTL/HiDPI/GPU tetap manual. [Kontrak7g](testing-stage7g.md).

## Stage 7h: hardening properti, 20 suite dijalankan ulang tanpa perubahan produk

Stage7h hanya menambah property test native (910 Cargo). Tidak ada perubahan kode
produk, skrip GUI, atau workflow, sehingga **tidak ada suite ke-21**: ke-20 gate GUI
dijalankan ulang utuh pada commit ini sebagai regresi dan seluruhnya lulus.
28 badan exercise lama tetap AST-identical `bb37223`; bytes/koordinat/oracle/deadline/
refocus tidak berubah.

**Lulus** [CI 39898595716](https://github.com/Indrawan007/Hylite/actions/runs/39898595716),
`c2af096`, job120234676767, 23 Oktober2026 04:07:10–04:17:50 UTC /
12:07:10–12:17:50 Makassar. **910Cargo**, **394GUI** pada 20 suite
(361clipboard,12titles,21natural exits), artifact `12477769679`,
3.253.627bytes, SHA256
`11c3fd76f5e200e24e196271113e81a580f6ff842a238ee72e5b60cff9143934`, unexpired.

Temuan yang dicatat: `text_input` Iced 0.13 menangkap Enter/PageUp/PageDown/Escape
saat fokus, sehingga shortcut commit7g hanya mungkin untuk draft `text_editor`.
[Kontrak7h](testing-stage7h.md).

## Stage 7i: konteks draft dan umpan balik blokir, 20 suite dijalankan ulang

Hanya teks header panel dan pesan status yang berubah; tombol, tinggi baris, dan
koordinat tetap. Tidak ada suite ke-21. Ke-20 suite (**394 checks**) dan 28 badan
exercise lama (AST-identical `bb37223`) lulus ulang pada commit `d22c7f3` —
[CI 39901301577](https://github.com/Indrawan007/Hylite/actions/runs/39901301577),
04:37:25–04:47:53 UTC, **921Cargo**, artifact `12479306282`, 3.238.614bytes,
SHA256 `60eeac0c449c98f69ae5e1ce94f142e3c169ff1a991dd32fdd1d711ae9787adb`.
[Kontrak7i](testing-stage7i.md).

## Stage 7j: suite ke-21 `paragraph-navigation-single` + 20 suite lama dijalankan ulang

Stage7j menambah dua baris tombol di bawah baris `Apply`/`Cancel` pada panel paragraf
satu baris (header, provenance20px, hint, input, dan baris `Apply`/`Cancel` tidak
bergeser), sehingga koordinat suite lama tidak berubah — **seluruh 20 gate lama
dijalankan ulang utuh** dan lulus. 28 badan exercise lama tetap AST-identical
`bb37223`.

Suite baru berjalan di Xvfb privat terpisah dengan alarm180s dan memakai tiga root
eligible yang dipisahkan heading, quote, code fence, dan list:

| Langkah | Oracle |
|---|---|
| Buka root pertama | payload raw persis + fokus input native |
| `Next` dua kali | melewati heading/quote/fence/list, tiba di root kedua lalu ketiga |
| Paste Unicode dua spasi lalu `Apply & Previous` | perubahan tetap lokal sampai Apply |
| `Apply & Previous` (dua kali) | commit payload ketiga, lalu root tengah/pertama unchanged tanpa edge |
| Tombol boundary pertama (disabled) + `Esc` | Source tidak berubah, hanya edge ketiga yang ada |
| Undo/Redo Source | tepat satu edge per commit |
| `# heading` + `Apply & Next` di Split | parser refusal, draft+fokus dipertahankan |
| `final é🙂` + `Apply & Next` | commit root pertama, buka root tengah sebagai draft baru |
| `Esc` lalu ketik `X` | fokus/caret Source kembali ke akhir payload yang baru diterapkan |
| Empat Undo | ketikan, Apply & Next, Apply & Previous, lalu welcome tanpa edge tersembunyi |

Dua screenshot (`single-paragraph-navigation-refused.png`,
`single-paragraph-navigation-split.png`) tanpa asersi piksel. `Ctrl+PageUp` sengaja
diuji sebagai **bukan** rute commit: `text_input` Iced0.13 menangkap
`Enter`/`Escape`/`PageUp`/`PageDown` saat fokus.

**Lulus** [CI 39906647794](https://github.com/Indrawan007/Hylite/actions/runs/39906647794),
`ecb6693`, job120260038439, 23 Oktober2026 05:35:58–05:47:04 UTC /
13:35:58–13:47:04 Makassar. **938 Cargo**, **21 suite / 419 pemeriksaan GUI**
(385 clipboard — 289 Source + 96 draft input; 12 title konfirmasi; 22 exit native
normal). Artifact `12481318417`, 3.315.303bytes, expires 2026-10-30T05:47:00Z;
SHA256 tidak dapat diunduh dari sandbox ini. [Kontrak7j](testing-stage7j.md).

Batas tetap: bukan bukti Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU, screenshot bukan
oracle piksel, dan navigasi hanya paragraf satu baris ↔ paragraf satu baris.
