# Konfirmasi dokumen dan lifecycle jendela — smoke X11

## Hasil yang benar-benar diamati

**Lulus**, [CI run 39713905090](https://github.com/Indrawan007/Hylite/actions/runs/39713905090),
commit `d03197b32c54988476d1617bd1f876c70560f127`, Ubuntu 24.04.
Build debug, Clippy `-D warnings`, **320 tes Cargo**, **14 tes Python helper**
dan **63 pemeriksaan GUI/lifecycle** lulus pada run yang sama. Total GUI terdiri
dari 49 perbandingan clipboard, 12 acknowledgment title konfirmasi, dan 2 exit
native normal; bukan 63 unit test Rust tambahan. Tes portable 150 tetap subset
320 tes Cargo.

[Artifact screenshot/log/summary](https://github.com/Indrawan007/Hylite/actions/runs/39713905090/artifacts/12398441652)
memuat bukti selama retensi 7 hari. Run pertama perluasan menemukan kehilangan
fokus setelah Cancel; pemulihan input telah diperbaiki dan diuji tanpa klik
penyelamat. Regresi lanjutan juga memastikan akselerator Alt tidak masuk sebagai
huruf `ds` ke Source di luar prompt.

## Metode

Ini perluasan `scripts/test-gui-x11.sh`: aplikasi Hylite sebenarnya dijalankan
pada Xvfb privat dengan renderer CPU tiny-skia, bukan callback controller tiruan.
Lihat [runner dan isolasi](testing-gui-x11.md). Semua dokumen adalah fixture tetap,
konfigurasi/cache memakai direktori sementara, dan tidak ada file pengguna dibuka.

Permintaan tutup dikirim sebagai **`WM_PROTOCOLS / WM_DELETE_WINDOW`** ke window
aplikasi menggunakan Python Xlib. Window harus mengiklankan protokol itu terlebih
dahulu. Ini jalur client close yang biasa dikirim window manager, bukan
`XDestroyWindow`, `xdotool windowclose`, SIGTERM atau `kill` untuk membuat tes lulus.
Tidak ada window manager/Hyprland di runner, jadi ini bukan tes binding Alt+F4
atau compositor-close pada sesi Wayland.

## Alur yang diperiksa

1. Paste fixture UTF-8/tab akhir, lalu Ctrl+N. Title harus menunjukkan
   `[confirm New]` sambil dirty tetap ada. Ketikan `LEAK`, undo/redo, Find,
   Go to Line, Open dan Ctrl+Alt+D selama prompt tidak boleh mengganti dokumen
   atau aksi yang menunggu. Setelah Escape, ketik `X` tanpa klik: teks harus
   tepat fixture + `X`, lalu satu undo mengembalikan fixture.
2. Ulangi Cancel dari Find dan Go to Line. Query harus mempertahankan nilainya
   dan langsung menerima keyboard lagi, bukan mengirim huruf ke dokumen.
   Tutup panel dan bandingkan seluruh source dengan fixture asli.
3. Ctrl+N lalu **Alt+D** mengganti dokumen dirty dengan dokumen baru clean.
   Ketik `Fresh` tanpa klik; clipboard harus persis `Fresh`, tidak ada teks
   dokumen lama. Alt+D/Alt+S di luar prompt tidak menyimpan/membuang dokumen
   atau menyisipkan `ds` melalui fallback editor.
4. Kirim WM_DELETE_WINDOW ketika dirty. Title harus `[confirm Close]`, bukan
   proses langsung mati. Ketikan selama konfirmasi diabaikan. Escape lalu
   ketik `Z` tanpa klik harus menghasilkan `FreshZ`; undo kembali ke `Fresh`.
5. Kirim close lagi dan pilih Discard via Alt+D. Proses harus keluar alami
   dengan **exit code 0** dalam timeout. Signal termination, exit nonzero atau
   timeout menggagalkan tes. Cleanup paksa tidak dihitung sebagai keberhasilan.
6. Jalankan proses baru yang clean dan verifikasi welcome. Kirim native close
   tanpa edit: harus keluar alami 0 tanpa memerlukan konfirmasi/discard.

Perbandingan source tetap byte-exact dan clipboard diganti marker sebelum Copy.
Tidak ada klik untuk menyelamatkan fokus pada alur Cancel/New/panel tersebut.
Title konfirmasi adalah status UI produksi yang juga terlihat di window list,
bukan flag test-only atau pengganti eksekusi dialog/app yang sebenarnya.

## Perbaikan aplikasi

- Title membedakan `[confirm New]`, `[confirm Open]` dan `[confirm Close]`, tanpa
  menghapus indikator dirty atau Zen.
- Konfirmasi menampilkan **Alt+S: Save & Continue**, **Alt+D: Discard & Continue**,
  dan **Escape: Cancel**. Aksi Save/Discard diabaikan controller jika tidak ada
  prompt; Discard/Cancel diblokir selama operasi file aktif. Exact bare Alt
  mengecualikan Ctrl+Alt, Shift+Alt dan logo; input AltGr tetap melalui native
  fallback jika bukan app command. Alt+D bersifat destruktif bila prompt aktif.
- Cancel membangun ulang widget tree editor sehingga fokus lama tidak dapat
  diandalkan. `restore_input_focus` memilih query Go to Line, query Find, atau
  Source sesuai konteks yang masih terbuka; Preview tetap tidak menjadi editor.
  New juga memulihkan fokus setelah penggantian dokumen. Tidak mengubah source,
  caret, snapshot, atau checkpoint hanya untuk mengembalikan fokus.
- Editor menyaring app shortcut sebelum fallback native, termasuk akselerator
  konfirmasi berbasis Alt, supaya satu tombol tidak juga menyisipkan karakter.

## Menjalankan dan membaca bukti

Dependensi Ubuntu tambahan untuk runner adalah `python3-xlib`:

```sh
sudo apt-get install --no-install-recommends -y python3-xlib
cargo build --locked
./scripts/test-gui-x11.sh --artifacts "$(mktemp -d)"
```

Wrapper selalu membuat display Xvfb terpisah. Jangan mengirim protokol close
ke DISPLAY sesi pribadi dengan menjalankan helper secara manual.

Artifact CI `gui-x11-smoke` berisi `summary.json`, `application.log`, screenshot
Source/navigasi dan `confirmation-new.png` / `confirmation-close.png`. Field
`native_exits` menghitung exit alami yang benar-benar diamati sebelum fallback
cleanup. Artifact berumur 7 hari, bukan bukti permanen di Git.

Unit test Python memeriksa bentuk client message, kegagalan bila protokol tidak
ada, dan penolakan exit signal/nonzero/timeout. Unit test Rust memeriksa title,
guard aksi tanpa prompt, konteks panel saat Cancel, exact-Alt vs Ctrl+Alt dan
pencegahan fallback penyisipan karakter. Unit test tersebut tidak menggantikan
smoke lifecycle yang benar-benar menjalankan task window-close.

## Batas bukti

- **Save & Continue di dalam prompt, Save As, Open file chooser, portal/D-Bus,
  penulisan dokumen lewat GUI, kegagalan save, dan close saat save in-flight
  belum diuji end-to-end di runner ini.** Alt+S di luar prompt bukan bukti save.
- Clipboard/teks dan lifecycle client X11 diuji, bukan seluruh state ID/history
  asinkron, event race, pemulihan crash, power loss atau recovery.
- Screenshot disimpan untuk inspeksi; tidak ada OCR maupun pixel assertion.
- Wayland/Hyprland, Alt+F4/compositor close, GPU, IME/RTL, Split/Zen, scrolling
  serta portal desktop target tetap perlu [checklist Tahap 2](testing-stage2.md)
  dan [Tahap 4](testing-stage4.md). Checklist manual tidak otomatis dicentang.
