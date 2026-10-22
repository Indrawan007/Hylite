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
dialog/window yang dikembalikan. Tidak ada jendela atau sesi Wayland yang diuji.

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
  dialog portal, D-Bus, close-window, keyboard compositor dan save end-to-end dari
  jendela nyata belum diuji. Tes file sementara bukan simulasi power-loss/recovery.
- Fokus, selection/IME/RTL, shaping/soft-wrap, syntax rendering, scrolling/caret
  viewport dan shortcut yang mungkin diambil Hyprland masih membutuhkan desktop.
- Batas algoritme Source, history snapshot O(n), LF canonical, selection collapse,
  gutter yang ditunda, dan fitur Markdown yang belum lengkap tetap berlaku.
- Jalankan [checklist Tahap 2](testing-stage2.md), [Tahap 3](testing-stage3.md),
  dan [Tahap 4](testing-stage4.md) pada salinan dokumen dan konfigurasi sementara.
  Jangan mencentang skenario desktop hanya karena suite Cargo lulus.

Gate build/lint/tes native kini terpenuhi. **Tahap 4 belum dinyatakan selesai dan
Tahap 5 renderer/WYSIWYG belum dimulai.** Tidak ada merge ke `main` dalam verifikasi ini.
