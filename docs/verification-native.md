# Verifikasi build dan tes native — 22 Oktober 2026

## Bukti yang diamati

[GitHub Actions run 39707517350](https://github.com/Indrawan007/Hylite/actions/runs/39707517350)
**success**, job **Linux build, lint, and tests**, pada commit
`373e05584a070049c52ab515395db1575ce52baf` di branch `arena/21a4c00b-hylite`.
Runner Ubuntu 24.04; toolchain Rust stable melalui workflow, bukan pin versi compiler.

| Pemeriksaan | Hasil |
| --- | --- |
| `cargo fmt --all -- --check` dan format harness | Lulus |
| Harness core dengan `clippy-driver`, `-D warnings` | 150 passed, 0 failed |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Lulus |
| `cargo build --locked --all-targets --all-features` | Lulus, profil debug/dev |
| `cargo test --locked --all-targets --all-features` | 312 passed, 0 failed, 0 ignored |
| Tes Python helper diagnostik CI | 3 passed |

**150 tes core termasuk dalam 312 tes Cargo**, lalu juga dijalankan secara mandiri;
angka tersebut bukan 462 tes Rust unik. Suite Cargo mengompilasi dan menguji kode
aplikasi dengan dependensi asli Iced, parser Markdown, konfigurasi dan filesystem.
Tes filesystem memakai direktori sementara; tes controller tidak menjalankan task
dialog/window yang dikembalikan. Tidak ada jendela atau sesi Wayland yang diuji **dalam run headless awal ini**.

Update runtime berikutnya: [CI run 39710774126](https://github.com/Indrawan007/Hylite/actions/runs/39710774126)
pada `ee73f2b` juga lulus, termasuk **smoke GUI aplikasi sebenarnya** melalui
X11/Xvfb + tiny-skia dan 9 tes Python helper. Lihat [bukti GUI terpisah](testing-gui-x11.md);
pengujian desktop Wayland/Arch/Hyprland tetap belum dilakukan.

Update navigasi/fokus: [CI run 39711916704](https://github.com/Indrawan007/Hylite/actions/runs/39711916704),
commit `72835cb`, lulus **315 tes Cargo**, 11 tes Python helper dan 34 pemeriksaan
GUI X11 (23 di antaranya tanpa refocus mouse). Tiga tes Cargo tambahan menguji
operasi fokus Source: idempoten, scope tidak menyentuh widget lain, dan scope
hilang adalah no-op. Smoke tanpa klik menemukan kehilangan fokus setelah undo;
operasi `focus_next` diganti dengan fokus idempoten dalam subtree container
Source. Lihat detail dan batas buktinya di [catatan GUI](testing-gui-x11.md).

Update lifecycle: [CI run 39713905090](https://github.com/Indrawan007/Hylite/actions/runs/39713905090),
commit `d03197b`, lulus **320 tes Cargo**, **14 tes Python helper**, serta **63
pemeriksaan GUI/lifecycle**. Lima regresi Rust tambahan mencakup shortcut
konfirmasi, title pending, no-op tanpa prompt, konteks Cancel dan pencegahan
Alt accelerator jatuh ke penyisipan karakter. Native close dirty + Discard dan
clean close benar-benar menghasilkan exit 0 sebelum fallback cleanup, bukan
SIGTERM. Lihat [bukti dan batas lifecycle](testing-lifecycle-x11.md). Portal file
serta close ketika save in-flight tetap belum diuji end-to-end.

Update renderer Stage 5a: [CI run 39716266646](https://github.com/Indrawan007/Hylite/actions/runs/39716266646),
commit `ea182b1`, lulus **347 tes Cargo**, **15 tes helper Python**, **67
pemeriksaan GUI X11**, build debug/format/Clippy strict. Tambahan 27 tes Rust
menguji model owned block/inline, adapter Iced dan invariant Source saat view
berubah; 4 pemeriksaan clipboard baru menguji Preview read-only dan round-trip
Source/Preview/Split tanpa perubahan/undo tambahan. 150 core adalah subset 347.
Screenshot bukan pixel assertion; model range belum menjadi source↔caret map.
Lihat [detail, artifact dan batas Stage 5a](testing-stage5a.md).

## Perbaikan yang dipicu CI

1. Clippy menolak task Iced `must_use` yang diabaikan tes controller. Tes sekarang
   menulis `let _ = ...` secara eksplisit; tidak ada penekanan lint dan task dialog
   tetap tidak dieksekusi oleh tes simulasi controller.
2. Setelah lint/build lulus, satu dari 309 tes gagal: undo empty-item Enter tidak
   memulihkan tab akhir. Akar masalahnya adalah `cosmic-text 0.12.1` `insert_at`
   yang membuang suffix `char::is_control` di segmen Paste, termasuk tab akhir,
   bukan hanya delimiter LF. Ekspektasi tes lama **tidak dilonggarkan**.
3. `document/state.rs::paste_exact` melindungi Paste dengan satu spasi sementara,
   lalu native Backspace menghapus hanya spasi itu. Load, rekonstruksi caret
   suffix/prefix, dan paste pengguna memakai helper yang sama. Operasi internal
   tidak membuat entri history/revisi tambahan; paste tetap satu transaksi.
4. Tiga regresi native ditambahkan: load trailing tab/LF, caret pada setiap split
   UTF-8 fixture bertab, dan paste pengganti seleksi dengan undo/redo atomik.
   Total suite menjadi 312; semuanya lulus, termasuk tes lama yang menemukan bug.
5. `scripts/ci-cargo.sh` mempertahankan exit status Cargo dan melaporkan diagnostik
   terbatas lewat GitHub Checks. Ini memungkinkan pembacaan hasil ketika unduhan
   arsip log Actions tidak dapat diakses dari lingkungan penyuntingan. Helper
   reporter diuji untuk escaping, command suspension, batas output, dan exit status.

## Batas verifikasi dan langkah berikutnya

- Ini build debug dan tes headless Ubuntu, **bukan** kelulusan desktop Arch Linux +
  Hyprland, bukan build release/AUR/packaging, dan bukan jaminan kesiapan produksi.
- Callback hasil file/controller, pemetaan binding dan model native diuji langsung;
  dialog portal, D-Bus, keyboard compositor dan save end-to-end dari
  jendela nyata belum diuji. Native client close X11 kini diuji pada suite lifecycle,
  bukan WM/compositor close atau close saat save berjalan. Tes file sementara bukan simulasi power-loss/recovery.
- Fokus, selection/IME/RTL, shaping/soft-wrap, syntax rendering, scrolling/caret
  viewport dan shortcut yang mungkin diambil Hyprland masih membutuhkan desktop.
- Batas algoritme Source, history snapshot O(n), LF canonical, selection collapse,
  gutter yang ditunda, dan fitur Markdown yang belum lengkap tetap berlaku.
- Jalankan [checklist Tahap 2](testing-stage2.md), [Tahap 3](testing-stage3.md),
  dan [Tahap 4](testing-stage4.md) pada salinan dokumen dan konfigurasi sementara.
  Jangan mencentang skenario desktop hanya karena suite Cargo lulus.

Gate build/lint/tes native baseline terpenuhi. **Tahap 4 belum dinyatakan selesai.**
Stage 5a renderer read-only kini mulai diimplementasikan; hasil terbaru dan batasnya
dipisahkan di [catatan Stage 5a](testing-stage5a.md). WYSIWYG tetap belum ada.
Tidak ada merge ke `main` dalam verifikasi ini.
