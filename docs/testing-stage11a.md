# Stage 11a — laporan diagnostik lingkungan

Stage ini menambahkan diagnostik yang diminta untuk membantu pemeriksaan target
native tanpa menganggap petunjuk environment sebagai bukti bahwa backend atau
portal tertentu benar-benar dipakai.

## Fitur

- Tombol **Copy env** pada toolbar membuat satu laporan saat diminta pengguna,
  mengirimkannya ke clipboard, dan mencatat laporan yang sama ke `stderr`.
- Tipe `iced::Renderer` yang dipakai build dicantumkan sebagai informasi
  compile-time. Scale factor ditanyakan dari window Iced saat dibuka dan setelah
  resize; nilai tidak valid dibuang.
- Laporan app menyertakan versi Hylite, target OS/arsitektur, batas versi
  dependency Iced, tipe renderer, scale factor window, font/ukuran dari
  konfigurasi Hylite, allowlist petunjuk locale dan sesi desktop, serta
  keberadaan display/D-Bus endpoint tanpa nilainya.
- `scripts/report-environment.sh` mencetak OS/kernel/arsitektur, versi ringkas
  toolchain bila tersedia, allowlist yang sama, hint keluarga fontconfig, dan
  status terbatas tiga service portal melalui `systemctl --user` (timeout tiga
  detik). Script menandai bahwa tipe renderer Iced hanya tersedia dari laporan
  app. Script tidak membutuhkan sesi grafis untuk dijalankan.

## Batas interpretasi

- Tipe renderer adalah tipe compile-time; itu **bukan** identifikasi GPU, adapter,
  API grafis aktif, atau backend window runtime.
- Aplikasi tidak mengintrospeksi provider file-dialog yang dipilih `rfd`.
  `systemctl` hanya melaporkan state service portal dan tidak membuktikan bahwa
  service tersebut dipilih atau berhasil dipakai oleh dialog.
- `WAYLAND_DISPLAY`, `DISPLAY`, dan D-Bus hanya dilaporkan set/unset. Nama socket,
  alamat bus, serta path runtime tidak disalin.
- Font aplikasi adalah konfigurasi yang diminta Hylite, bukan pembuktian font
  final yang dipilih renderer. `fc-match monospace` pada script adalah hint
  fontconfig terpisah.
- Belum ada verifikasi manual di Arch + Hyprland/Wayland. Stage 11a tidak
  menggantikan checklist Stage 11b dan tidak membuktikan dialog portal, IME,
  RTL, HiDPI, atau GPU.

## Privasi laporan

App hanya membaca `LC_ALL`, `LC_MESSAGES`, `LANG`, `LANGUAGE`,
`XDG_SESSION_TYPE`, `XDG_CURRENT_DESKTOP`, `XDG_SESSION_DESKTOP`, serta status
keberadaan `WAYLAND_DISPLAY`, `DISPLAY`, dan `DBUS_SESSION_BUS_ADDRESS`. Nilai
teks dibatasi ke karakter label yang aman; nilai yang mengandung path, control
character, atau format tak dikenal ditampilkan sebagai `[filtered]`. Script
menggunakan allowlist yang sama dan tidak melakukan `env`/`printenv`.

Kedua laporan tidak mengakses atau menyertakan isi dokumen, path dokumen, home,
`XDG_RUNTIME_DIR`, display/socket value, maupun alamat D-Bus. Service portal
hanya dipanggil dengan nama unit tetap; output/error service selain state yang
terbatas tidak ditampilkan. Salin laporan tetap tindakan eksplisit pengguna.

## Pengujian

- Unit test Rust mengunci field laporan, filter nilai berbahaya, skala valid/tak
  valid, dan keberadaan endpoint yang hanya dilaporkan sebagai boolean.
- `scripts/test_environment_report.py` memanggil script dalam environment
  terkontrol, memakai stub `systemctl`/`fc-match`, dan menguji allowlist,
  redaksi path/alamat, serta penolakan nilai multiline.
- Pemeriksaan lokal: seluruh 98 tes helper Python lulus,
  `bash -n scripts/report-environment.sh` lulus, dan script berjalan di sandbox.
- Revisi kode `fdf991b` lulus seluruh job di [CI `40244045608`](https://github.com/Indrawan007/Hylite/actions/runs/40244045608): helper Python, format, strict Source core, Clippy, build, tes Rust, serta semua suite GUI X11 dan probe opt-in yang sudah ada. Ini bukan bukti verifikasi Arch + Hyprland/Wayland, bukan verifikasi portal runtime, dan bukan release/package build.
