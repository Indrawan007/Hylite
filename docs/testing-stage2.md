# Checklist pengujian desktop — Tahap 2

**Status:** belum dijalankan pada sesi Arch + Hyprland. Checklist ini bukan laporan kelulusan. Gunakan dokumen percobaan dalam direktori sementara, bukan dokumen penting.

Bukti otomatis terpisah: [smoke lifecycle X11](testing-lifecycle-x11.md) sudah lulus untuk New/Cancel/Discard, pemulihan fokus input serta native close dirty/clean. Ini tidak mencentang checklist manual Arch/Hyprland di bawah. Save & Continue/Save As/Open melalui portal dan penulisan ke disk dari GUI masih belum diuji end-to-end.

## Sebelum memulai

Pada lingkungan dengan toolchain dan akses crates.io penuh:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo run --locked
```

Pastikan file chooser portal berjalan di sesi desktop. Ulangi kasus yang relevan pada Source, Split, Preview, dan Zen. Unit test state tidak mengeksekusi native dialog atau benar-benar menutup jendela.

## Save dan Save As

- [ ] Buat dokumen baru, ketik, lalu Save. Pilih `first.md`. Nama pada title/sidebar/status bar berubah dan dirty hilang setelah penulisan berhasil.
- [ ] Edit kembali dan Save. Tidak ada dialog kedua; `first.md` berisi hasil baru.
- [ ] Gunakan tombol Save As dan `Ctrl+Shift+S` untuk menyimpan ke `second.md`. File pertama tetap utuh; Save berikutnya menulis file kedua.
- [ ] Batalkan Save As. Path lama, isi buffer, dan status dirty tidak berubah.
- [ ] Coba menyimpan ke file read-only atau lokasi tanpa izin. Penyebab error harus terlihat, termasuk dalam Zen; isi file asli tetap utuh.
- [ ] Pada dokumen besar/save yang lambat, ketik lagi sebelum save selesai. Penanda dirty harus tetap aktif; Save ulang menyimpan perubahan terbaru.

## New / Open / Close saat dirty

Untuk **masing-masing** aksi New, Open, dan tombol tutup jendela/window-manager close:

- [ ] Cancel atau `Esc` mempertahankan isi, path, dan dirty. `Esc` pada prompt di Zen tidak keluar dari Zen.
- [ ] Save & Continue meminta lokasi bila untitled, menulis dokumen lama, baru menjalankan aksi. Untuk Close, jendela harus tetap hidup sampai penulisan berhasil.
- [ ] Batalkan dialog save. Aksi tidak dilanjutkan; prompt tetap tersedia.
- [ ] Buat save gagal (misalnya tujuan read-only). Aksi tidak dilanjutkan; prompt memperlihatkan penyebab dan dapat dicoba ulang/dibatalkan.
- [ ] Discard & Continue menjalankan aksi tanpa menyimpan perubahan lama.
- [ ] Coba shortcut New/Open/Export ketika prompt tampil. Tidak muncul operasi/dialog kedua dan aksi yang menunggu tidak berubah.

Tambahan Open:

- [ ] Setelah Discard & Continue → Open, batalkan file chooser. Dokumen lama dan dirty tetap tersedia.
- [ ] Buka file bukan UTF-8 atau file yang tidak dapat dibaca. Error bukan dianggap pembatalan; buffer lama tidak diganti.
- [ ] Saat pembacaan file berlangsung, editing tidak mengubah buffer lama. Hasil sukses mengganti path dan isi sekaligus.

## Ekspor HTML

- [ ] Ekspor dokumen yang belum disimpan. HTML terbentuk; buffer tetap dirty dan path Markdown tidak berubah.
- [ ] Batalkan ekspor dan coba ekspor ke lokasi tidak valid. Status cancel/error berbeda; tidak mengubah status penyimpanan Markdown.
- [ ] Pilih file Markdown aktif sebagai tujuan, termasuk melalui alias symlink. Ekspor harus ditolak dan isi sumber tetap Markdown.
- [ ] Edit selama ekspor berjalan. Hasil HTML merupakan snapshot awal; perubahan buffer tetap dirty.

## Penulisan atomik dan metadata

- [ ] Simpan file dengan mode `0640`, lalu periksa bahwa mode tetap `0640`.
- [ ] Simpan file baru; mode harus `0600` (privat).
- [ ] Buka/simpan melalui symlink relatif yang valid. Link tetap berupa symlink dan target diperbarui.
- [ ] Coba Save As ke symlink putus atau file dengan hard-link. Operasi harus gagal dengan pesan, tanpa mengganti link/file asli.
- [ ] Pastikan tidak ada `.hylite-*` tersisa setelah operasi normal/gagal yang sudah selesai. Force-kill dapat meninggalkan file sementara dan belum memiliki recovery.
- [ ] Ingat bahwa file dengan ACL/xattr/ownership khusus belum didukung sepenuhnya; jangan gunakan sebagai dokumen produksi.

## Operasi bertumpuk dan penutupan

- [ ] Selama operasi file masih berjalan, tekan Save/New/Open/Close berulang. Operasi tidak bertumpuk; aplikasi meminta menunggu. Setelah selesai, ulangi aksi yang diinginkan.
- [ ] Tutup dokumen bersih. Jendela dan proses aplikasi benar-benar berakhir.
- [ ] Tutup dokumen dirty setelah Save berhasil dan setelah Discard. Jendela dan proses aplikasi benar-benar berakhir; Cancel tidak menutup aplikasi.
- [ ] Konfirmasi juga muncul dalam Zen dan melalui perintah close Hyprland biasa. Force-kill berada di luar perlindungan ini.

## Cakupan pengujian otomatis yang ditambahkan

- Transisi state save/export, revisi yang berubah, hasil operasi kedaluwarsa, dan pencegahan operasi bertumpuk.
- Guard New/Open/Close, save-sebelum-aksi, cancel/error, serta pemblokiran input saat prompt/Open.
- Penulisan atomik, kegagalan tulis parsial, pembersihan temporary file, permission, symlink, hard-link, dan proteksi ekspor.
- Worker I/O save tanpa dialog serta pelestarian penyebab error.

Build dan hasil test harus dicatat setelah benar-benar dijalankan. Kelulusan formatter/metadata saja bukan bukti kelulusan build atau tes desktop.
