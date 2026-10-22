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
