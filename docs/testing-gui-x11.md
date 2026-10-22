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

Workflow menjalankan smoke sesudah build dan 320 tes Cargo. Output ringkas
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
dan exit alami vs signal/timeout. Bersama tiga tes reporter CI, ada **14 tes
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
