# Checklist Tahap 3 — model dan konfigurasi

**Status desktop: belum dijalankan.** Build debug, Clippy dan suite Cargo (312 tes, termasuk tracker operasi/config/controller) sudah lulus di CI Ubuntu 24.04 / Rust stable. Lihat [catatan verifikasi native](verification-native.md). Kelulusan headless tidak membuktikan perilaku desktop.

## Pemeriksaan otomatis pada lingkungan lengkap

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
```

Tes tambahan mencakup default per field, round-trip TOML, enum, range/NaN/infinity, font, unsupported autosave, penanganan file konfigurasi tanpa overwrite, invariant revisi, tracker operasi, binding Tab, serta pemisahan warning/error. Regresi state dari Tahap 2 tetap dipertahankan dan dimigrasikan ke API dokumen.

## Pengujian desktop

Gunakan `XDG_CONFIG_HOME` sementara agar tidak mengubah preferensi asli:

```sh
scratch=$(mktemp -d)
mkdir -p "$scratch/hylite"
# Tulis contoh konfigurasi ke "$scratch/hylite/config.toml".
XDG_CONFIG_HOME="$scratch" cargo run --locked
```

Environment D-Bus/Wayland/Hyprland sesi desktop tetap diperlukan. Perubahan konfigurasi memerlukan restart aplikasi.

- [ ] Jalankan tanpa file konfigurasi. Default dibuat; font source monospace, ukuran 16, Source mode, tema Catppuccin Mocha. File konfigurasi baru memiliki mode `0600`.
- [ ] Isi hanya `[editor]` dengan `font_size = 20.0` dan `default_mode = "split"`. Font size dan mode berubah; field/section lain tetap default.
- [ ] Coba font custom yang memang terpasang. Source dan sisi source Split memakai font/ukuran yang sama. Tidak ada pengujian keberadaan font otomatis; text engine menentukan fallback.
- [ ] Uji `word_wrap = true` dan `false` pada paragraf panjang di Source maupun Split. Pastikan wrapping berubah tanpa mengganti isi dokumen. Perilaku navigasi horizontal untuk baris panjang tetap perlu ditinjau.
- [ ] Atur `tab_size = 4`: setelah satu karakter, Tab menyisipkan tiga spasi; di kolom awal/kelipatan empat, empat spasi. Setelah restart dengan nilai 2, stop mengikuti kelipatan dua.
- [ ] Uji Tab saat teks dipilih, serta Tab saat prompt unsaved/Open aktif. Editing biasa mengikuti perilaku paste native; prompt/Open tidak boleh mengubah buffer.
- [ ] Paste literal tab. Karakter tab dipertahankan; setting hanya mengatur spasi yang disisipkan melalui binding Tab, bukan lebar visual literal tab.
- [ ] Uji input biasa, Ctrl+S, Ctrl+Shift+S, copy/paste, seleksi, dan pergerakan kursor. AltGr/Ctrl+Alt tidak boleh meluncurkan operasi file.
- [ ] Set `font_size = nan`, `tab_size = 0`, dan lebar di luar range bersamaan dengan `theme = "nord"`. Warning harus tampil; field invalid kembali ke default dan tema valid tetap Nord. File asli tidak ditulis ulang.
- [ ] Coba key salah, mode/tema salah, TOML rusak, atau tipe field salah. Aplikasi memakai seluruh default dan memperlihatkan path serta penyebab. File invalid tetap utuh.
- [ ] Coba konfigurasi lama dengan `appearance.opacity`. Warning menyebut key tak dikenal; hapus key tersebut untuk migrasi. Opacity dikelola melalui Hyprland.
- [ ] Set `auto_save = true`. Warning harus menjelaskan bahwa fitur belum tersedia; jangan menganggap tulisan disimpan otomatis.
- [ ] Pastikan warning terlihat pada Zen dan prompt unsaved. Dismiss warning tidak membersihkan error file maupun status dirty dokumen.
- [ ] Ulangi seluruh checklist [Tahap 2](testing-stage2.md), khususnya save sambil mengetik, Save As, save-before-close, dan ekspor saat dirty.

Tema terang masih memiliki elemen hardcoded gelap; itu batasan yang diketahui, bukan bukti pemetaan enum tema gagal.
