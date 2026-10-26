# Stage 11b — checklist manual Arch + Hyprland/Wayland

**Status:** template/instruksi saja; belum ada pemeriksaan runtime yang dijalankan
atau hasil target yang dicatat. Checklist ini harus diisi dari sesi desktop Arch
+ Hyprland/Wayland milik pengguna. CI X11/Xvfb tidak menggantikannya.

Persetujuan yang ada hanya mencakup persiapan checklist ini. Tidak ada perubahan
kode produksi, tidak ada verifikasi target yang diklaim, dan Stage 11c (PKGBUILD /
release build) tetap di luar scope.

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

| Data | Nilai |
|---|---|
| Tanggal/waktu lokal | — |
| Branch dan commit Hylite | — |
| Distro/versi, kernel, arsitektur | — |
| Sesi compositor dan jenis sesi yang dilaporkan | — |
| GPU dan driver (jika diketahui) | — |
| Monitor/scale factor yang dilaporkan app | — |
| Input method aktif (fcitx5, ibus, lain, atau tidak ada) | — |
| Renderer type compile-time dari Copy env | — |
| Hasil ringkas status service portal | — |

`Renderer type` pada laporan app adalah informasi compile-time, bukan identifikasi
GPU/API grafis runtime. Status `active` dari `systemctl` juga bukan bukti bahwa
file dialog memilih atau berhasil memakai portal tertentu.

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
| Session Wayland + Hyprland terkonfirmasi | NOT RUN | — |
| Portal: Open, Cancel, Save As, Cancel | NOT RUN | — |
| IME yang tersedia | NOT RUN | — |
| RTL / teks campuran | NOT RUN | — |
| HiDPI / scale factor | NOT RUN | — |
| GPU / render window dan Preview | NOT RUN | — |
| Tutup bersih dan prompt dirty | NOT RUN | — |

**Kesimpulan target:** belum diisi — tunggu bukti dari mesin Arch + Hyprland/Wayland.
Tuliskan kegagalan, kondisi yang tidak dapat diuji, dan batas reproduksi apa adanya.
Tidak ada hasil runtime yang disimpulkan dari template ini. Setelah hasil manual
tersedia, Stage 11b dapat ditinjau terpisah; Stage 11c tetap menunggu otorisasi
terpisah.
