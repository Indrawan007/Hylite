# Smoke test GUI Source pada X11 virtual

## Hasil yang benar-benar diamati

**Lulus**, [CI run 39710774126](https://github.com/Indrawan007/Hylite/actions/runs/39710774126),
commit `ee73f2b2c289a8de452d42e71866a44084f5c5c2`, Ubuntu 24.04.
Sebelas pemeriksaan Source/clipboard berhasil; build, Clippy `-D warnings`,
312 tes Cargo dan 9 tes Python helper juga lulus pada run yang sama.
[Artifact screenshot/log/summary](https://github.com/Indrawan007/Hylite/actions/runs/39710774126/artifacts/12397065634)
tersedia selama masa retensi. Jumlah pemeriksaan GUI tidak dijumlahkan sebagai
unit test Rust; 150 tes portable tetap merupakan subset 312 tes Cargo.

Percobaan startup pertama menemukan `libxkbcommon-x11` yang belum terpasang;
dependensi runtime CI dan petunjuk Arch telah diperbaiki. Runner juga diperbaiki
untuk menghindari write Copy tertunda, memberi jeda antar batch shortcut Iced,
dan membatasi thread serta memberi anggaran capture screenshot terpisah 15 detik.
Tidak ada perubahan pada logika editor Rust atau pelonggaran ekspektasi teks
untuk membuat smoke ini lulus.

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

## Cara menjalankan

Build aplikasi terlebih dahulu. Pada Ubuntu, dependensi smoke tambahan:

```sh
sudo apt-get install --no-install-recommends -y \
  xvfb xauth xdotool xclip imagemagick fonts-dejavu-core libxkbcommon-x11-0
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
saat selesai atau gagal. Cleanup proses **bukan** verifikasi close-window native.

## CI dan bukti

Workflow menjalankan smoke sesudah build dan 312 tes Cargo. Output ringkas
tersedia di GitHub Checks; artifact **`gui-x11-smoke`** di halaman run berisi:

- `summary.json`: status, pemeriksaan yang berhasil, dan error bila ada;
- `application.log`: stdout/stderr aplikasi dan clipboard owner;
- `tabs.png` / `empty-lines.png`: tangkapan jendela dengan fixture terpilih;
- `failure.png`: tangkapan best-effort jika window tersedia saat gagal.

Artifact memakai direktori segar di `RUNNER_TEMP` (bukan target cache) dan
retensi 7 hari. Tidak ada dokumen pribadi atau dump environment dalam fixture.
Kegagalan aplikasi, clipboard, screenshot, timeout atau komparasi menggagalkan
step; tidak ada `continue-on-error`. Anggaran global 120 detik, serta batas
per-command/wait dan timeout step CI 5 menit.

Enam unit test helper baru menguji isolasi environment, child exit, timeout,
komparasi byte-exact, pencegahan false-positive clipboard dan polling tanpa write Copy tambahan. Bersama tiga tes
reporter CI, ada **9 tes Python**; unit test helper ini **bukan** bukti runtime GUI.

## Batas bukti

Kelulusan smoke hanya mencakup jalur X11/tiny-skia dan skenario di atas.
Screenshot disimpan untuk inspeksi, **tidak** dibandingkan pixel-per-pixel;
keberadaan emoji di clipboard tidak membuktikan glyph dirender dengan benar.
Copy memfokuskan editor melalui klik eksplisit; pemulihan fokus otomatis setelah
Find/Go to Line/undo tidak dianggap terverifikasi oleh pemeriksaan tersebut.

Wayland/Hyprland, akselerasi GPU, portal file, D-Bus, close/save dari dialog,
IME/RTL, soft-wrap/viewport, mouse selection kompleks dan compositor shortcuts
masih membutuhkan [checklist desktop Tahap 4](testing-stage4.md) beserta
[Tahap 2](testing-stage2.md) dan [Tahap 3](testing-stage3.md). Tes ini tidak
menjadikan Tahap 4 selesai atau aplikasi siap menyimpan dokumen penting.
