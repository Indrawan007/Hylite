# Stage 11b — checklist manual Arch + Hyprland/Wayland

**Status:** checklist disiapkan dan preflight parsial dicatat dari output yang
diberikan pengguna. Build debug selesai dan laporan shell memberi petunjuk sesi
Arch/Wayland/Hyprland, tetapi belum ada hasil interaksi portal, IME, RTL, HiDPI,
GPU, atau lifecycle. Checklist tetap harus diisi dari sesi target milik pengguna;
CI X11/Xvfb tidak menggantikannya.

Catatan di bawah hanya merangkum output yang dibagikan dan tidak menyimpan path
lokal/alamat endpoint. Ini bukan klaim bahwa Stage 11b selesai. Tidak ada perubahan
kode produksi atau Stage 11c (PKGBUILD/release build).

## 1. Persiapan aman

1. Gunakan sesi **Hyprland pada Wayland** yang memang akan menjadi target; jangan
   gunakan Xvfb, sesi X11, atau aplikasi yang berjalan hanya melalui XWayland
   sebagai pengganti.
2. Bangun/jalankan checkout Hylite yang ingin diuji. Dari root repo:

   ```sh
   git branch --show-current
   git rev-parse --short HEAD
   cargo build --locked
   ./target/debug/hylite
   ```

   Catat branch, revisi, versi Rust, dan apakah aplikasi benar-benar diluncurkan
   dari sesi target. Build debug ini hanya sarana uji Stage 11b; bukan release
   build atau paket Arch.
3. Jalankan `./scripts/report-environment.sh` dari terminal di sesi yang sama.
   Di Hylite, tekan **Copy env** lalu tempel laporan ke editor teks lokal kosong
   untuk memeriksanya. Laporan Stage 11a menyamarkan nilai display/D-Bus dan path,
   tetapi tetap tinjau sendiri sebelum membagikan apa pun.
4. Jangan gunakan dokumen penting. Untuk alur dialog, siapkan fixture baru yang
   tidak memuat data pribadi. Contoh berikut menampilkan path hanya di terminal lokal:

   ```sh
   stage_dir="$(mktemp -d)"
   printf 'Stage 11b fixture\n\nMarker: HYLITE-11B-OPEN\n' > "$stage_dir/input.md"
   printf 'Fixture directory (local only; redact before sharing): %s\n' "$stage_dir"
   ```

   Path perlu diketahui untuk memilih fixture di dialog, tetapi jangan salin ke
   laporan/log yang dibagikan tanpa menyamarkannya. Gunakan file tersebut sebagai
   sumber dan simpan hasil uji ke file **baru** di direktori yang sama. Jangan
   menimpa dokumen yang sudah ada.

## 2. Catat lingkungan yang benar-benar diuji

Isi sebelum menjalankan checklist. Tulis `tidak diketahui` jika tidak dapat
memastikan suatu nilai; jangan menyimpulkannya dari nama paket atau service.

| Data | Nilai yang dilaporkan |
|---|---|
| Tanggal laporan diterima | 26 Oktober 2026; waktu uji tidak disertakan |
| Branch dan commit Hylite yang diuji | tidak disertakan |
| Distro/versi, kernel, arsitektur | Arch (`ID=arch`; `VERSION_ID` tidak tersedia pada laporan), Linux `7.2.11-arch1-1`, `x86_64` |
| Sesi compositor dan jenis sesi yang dilaporkan | `XDG_CURRENT_DESKTOP=Hyprland`, `XDG_SESSION_TYPE=wayland` |
| GPU dan driver (jika diketahui) | belum dilaporkan |
| Monitor/scale factor yang dilaporkan app | belum dilaporkan; belum ada output tombol **Copy env** |
| Input method aktif (fcitx5, ibus, lain, atau tidak ada) | belum dilaporkan |
| Renderer type compile-time dari Copy env | belum dilaporkan; skrip shell tidak dapat membacanya |
| Locale/fontconfig | `LANG=en_US.UTF-8`; monospace `Noto Sans Mono:style=Regular` |
| Endpoint display/D-Bus | `WAYLAND_DISPLAY`, `DISPLAY`, dan bus D-Bus set; nilai tetap disamarkan |
| Toolchain | `rustc 1.99.0`, `cargo 1.99.0` |
| Status service portal | ketiga unit yang dicetak (portal umum, Hyprland, GTK) dilaporkan `active` |

`Renderer type` pada laporan app adalah informasi compile-time, bukan identifikasi
GPU/API grafis runtime. Status `active` dari `systemctl` juga bukan bukti bahwa
file dialog memilih atau berhasil memakai portal tertentu. Kedua endpoint Wayland
serta X11 yang `set` tidak mengubah laporan jenis sesi `wayland` menjadi sesi X11;
nilai socket tetap tidak diketahui karena memang disamarkan.

### Hasil preflight yang dibagikan

- Build profile `dev` selesai dalam **30,80 detik** menurut output pengguna. Ini
  bukti build debug saja, bukan bukti release build atau perilaku GUI.
- Saat Hylite membaca konfigurasi lokal, aplikasi menolak key `opacity = 1.0`
  karena field itu tidak didukung, lalu menggunakan default. Hapus key tersebut
  dari konfigurasi lokal dan jalankan ulang sebelum menguji font/tema konfigurasi;
  jika ingin mengatur opacity, proyek mengarahkannya ke Hyprland. Path konfigurasi
  lokal sengaja tidak disimpan di catatan ini.
- Output warning menunjukkan konfigurasi ditolak, tetapi tidak secara eksplisit
  mencatat apakah window tampil atau tidak. Hasil interaksi aplikasi masih perlu
  dicatat terpisah.

## 3. Pemeriksaan manual

Untuk setiap item gunakan status **PASS**, **FAIL**, **BLOCKED**, atau **NOT RUN**.
Sertakan pengamatan ringkas; jangan mengubah `BLOCKED`/`NOT RUN` menjadi `PASS`.
Ulangi alur portal setelah meluncurkan app langsung dari terminal di sesi target.

### A. Portal dan file dialog

- [ ] Buka file `.md` fixture melalui tombol **Open**. Catat apakah dialog benar-benar
      berasal dari portal yang diharapkan, file terpilih, dan marker fixture muncul
      di Hylite. Jangan menganggap service aktif sebagai bukti dialog portal.
- [ ] Batalkan **Open**. Dokumen, path/title aktif, dan status dirty tidak berubah.
- [ ] Gunakan **Save As** untuk membuat file baru di direktori fixture. Pastikan
      dialog tampil, operasi selesai, file baru dapat dibuka lagi, dan marker yang
      diuji ada di salinan tersebut.
- [ ] Batalkan **Save As**. Tidak ada file tujuan baru dan status dokumen tidak
      berubah akibat pembatalan.
- [ ] Jika UI dialog tidak menunjukkan apakah portal dipakai, catat provider sebagai
      **tidak terkonfirmasi**; jangan menebak dari unit `xdg-desktop-portal*`.

### B. IME (fcitx5/ibus)

- [ ] Catat IME yang benar-benar aktif. Jika tidak tersedia, tandai **BLOCKED** dan
      catat alasannya; mengetik karakter yang sudah dikomposisi lewat paste bukan
      pengganti uji IME.
- [ ] Di Source, mulai komposisi IME, lanjutkan teks sebelum commit, lalu commit
      teks non-ASCII. Amati preedit/underline, posisi caret, karakter hilang/ganda,
      dan apakah teks setelah commit masuk di posisi yang benar.
- [ ] Batalkan satu komposisi dan coba commit lain. Pastikan pembatalan tidak
      menyisakan karakter parsial; catat perilaku aktual, termasuk jika ada masalah.
- [ ] Setelah komposisi, ketik teks Latin biasa dan uji Undo/Redo secara hati-hati
      pada fixture saja. Jangan mengasumsikan batas transaksi IME tanpa mengamatinya.

### C. RTL dan teks campuran

- [ ] Di fixture, uji teks Arab atau Ibrani yang memang dapat Anda ketik, dicampur
      dengan Latin, angka, tanda kurung dan tanda baca.
- [ ] Amati urutan visual, gerak caret kiri/kanan, seleksi, Backspace/Delete, dan
      posisi setelah baris dibungkus. Catat perilaku yang tampak; checklist ini tidak
      menetapkan dukungan RTL penuh atau oracle glyph/pixel.
- [ ] Pastikan teks asli tetap dapat disimpan dan dibuka lagi. Jangan memakai
      dokumen pribadi sebagai bahan uji.

### D. HiDPI / scale

- [ ] Catat scale factor dari **Copy env** setelah window dibuka.
- [ ] Jika perangkat/session memungkinkan, ulangi pada scale factor lain—idealnya
      satu skala normal dan satu skala tinggi/fraksional—dengan mengubah pengaturan
      melalui cara normal di mesin tersebut. Catat nilai yang benar-benar dilaporkan;
      jangan menganggap perubahan berhasil tanpa membacanya dari app.
- [ ] Pada setiap skala, periksa toolbar/status bar, teks Source dan Preview, tombol,
      clipping, caret/scroll, resize, dan keterbacaan. Pulihkan pengaturan monitor
      semula setelah tes.

### E. Render dengan GPU

- [ ] Luncurkan app pada konfigurasi GPU/driver yang memang dipakai sesi target.
      Catat GPU/driver dari tool lokal yang tersedia; jangan memasang tool atau
      mengubah driver hanya untuk menghasilkan klaim.
- [ ] Periksa apakah window ter-render (bukan kosong/berkedip), lalu resize, scroll,
      edit fixture, dan tampilkan Preview/Split. Catat error render/crash yang nyata.
- [ ] Jangan menyimpulkan backend/API GPU aktif dari `iced::Renderer` type name.
      Jangan menjadikan pemeriksaan ini klaim performa, stabilitas universal, atau
      pixel oracle.

### F. Tutup dan perubahan belum tersimpan

- [ ] Tutup app dengan dokumen bersih; catat apakah jendela keluar secara normal.
- [ ] Pada fixture saja, buat edit kecil lalu tutup jendela. Pastikan prompt perubahan
      belum tersimpan muncul; **Cancel** harus membiarkan app/dokumen tetap terbuka.
- [ ] Uji **Save** atau **Discard** hanya pada fixture. Pastikan hasil sesuai pilihan
      dan tidak ada dokumen penting yang tertimpa.

## 4. Lembar hasil

Salin tabel ini untuk setiap mesin/session yang diuji. Lampirkan screenshot/log hanya
setelah meninjau dan menyamarkan nama pengguna, home/runtime path, nama file sensitif,
isi dokumen, alamat D-Bus, token, dan informasi lain yang tidak perlu.

| Area | Status | Pengamatan / versi / scale yang diuji (tanpa data sensitif) |
|---|---|---|
| Environment report menunjukkan Wayland + Hyprland | PASS (hint environment saja) | `XDG_SESSION_TYPE=wayland`, `XDG_CURRENT_DESKTOP=Hyprland`; bukan validasi perilaku GUI |
| `cargo build --locked` pada profile dev | PASS (build saja) | selesai dalam 30,80 detik menurut output yang dibagikan |
| Portal: Open, Cancel, Save As, Cancel | NOT RUN | service portal `active`, tetapi dialog belum diuji |
| IME yang tersedia | NOT RUN | — |
| RTL / teks campuran | NOT RUN | — |
| HiDPI / scale factor | NOT RUN | laporan **Copy env** dari aplikasi belum diterima |
| GPU / render window dan Preview | NOT RUN | GPU/driver dan hasil visual belum dilaporkan |
| Tutup bersih dan prompt dirty | NOT RUN | — |

**Kesimpulan sementara:** preflight environment dan build debug berhasil menurut
output pengguna; checklist target belum selesai. Catat kegagalan, kondisi yang tidak
dapat diuji, dan batas reproduksi apa adanya. Jangan menyimpulkan portal berfungsi
dari status service. Stage 11b baru dapat ditinjau setelah interaksi manual dicatat;
Stage 11c tetap menunggu otorisasi terpisah.
