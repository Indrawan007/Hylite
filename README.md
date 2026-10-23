# Hylite

Editor Markdown native berbasis **Rust + Iced** untuk **Arch Linux + Hyprland**, dengan pengalaman MarkText sebagai acuan. Hylite tidak menggunakan Electron atau WebView.

> **Status: prototipe awal (0.1.0), belum siap untuk dokumen penting.** WYSIWYG umum ala MarkText belum tersedia; Stage 6a–7e baru prototipe checkbox task, level heading ATX/Setext, alignment/operasi baris-kolom, panel draft raw sel tabel/paragraf root dan Undo/Redo terbatas. Simpan salinan cadangan sebelum mencoba aplikasi.

## Fitur yang sudah diimplementasikan

- Editor Source, preview native terstruktur (Stage 5a–5e), dan Split view; teks preview tetap tidak editable; pengecualian terbatas adalah checkbox task, level heading root ATX/Setext, alignment, whole body row/column dan panel draft raw Markdown sel tabel/paragraf root melalui kontrol tervalidasi.
- Cache model preview umumnya lazy per revisi konten; renderer meminjam model dan edit/undo/redo membuang model lama. Apply & Next/Previous melakukan preflight parse dan memasang model baru setelah commit. Ini belum incremental parsing atau benchmark speedup.
- Tombol **Show block source** pada leaf preview membuka Split dan memfokuskan awal rentang provenance Source; target stale ditolak. Ini bukan glyph/selection mapping.
- **Klik kanan sel tabel → Source**: target harus cocok dengan tabel/baris/kolom/revisi saat ini; sel padding tanpa rentang sumber tidak mempunyai action. Klik kiri tetap read-only.
- **Prototipe 6a: checkbox task di Preview/Split** mengubah satu byte marker kanonis melalui transaksi Source Atomic. Target stale/forged ditolak; tidak men-serialize ulang AST.
- **Prototipe 6b–7e: Undo/Redo edit tervalidasi langsung di Preview**, lewat Ctrl+Z, Ctrl+Shift+Z/Ctrl+Y dan toolbar. Hanya transaksi checkbox/level heading/alignment/row/column/cell Apply tabel/paragraph Apply di puncak history; tidak melewati edit Source. Source/Split mempertahankan Undo/Redo umum.
- **Prototipe 6c: H−/H+ pada heading root ATX** di Preview/Split, dibatasi level 1–6. Hanya satu byte `#` pembuka berubah; isi raw/closing hashes/whitespace tetap utuh. Heading dalam container tetap tidak bisa diedit lewat kontrol ini.
- **Prototipe 6d: H−/H+ untuk heading Setext root** mengubah level 1↔2 hanya dengan mengganti run `=`/`-` pada underline. Panjang garis, indent, body multiline/raw dan whitespace tidak dinormalisasi; tidak dikonversi ke ATX.
- **Prototipe 6e: Check tasks / Clear tasks** menargetkan semua task kanonis pada model Preview/Split yang didukung, maksimal 1024, dalam satu transaksi Atomic (dalam budget history). Kapital `X` yang sudah checked tidak dinormalisasi; literal/marker unsupported tidak disentuh. No-op dan batch melebihi cap tidak diterapkan.
- **Prototipe 6f: alignment kolom** (dalam **Table actions…**) mengubah Default/Left/Center/Right pada kolom tabel GFM root yang kanonis di Preview/Split. Hanya colon token delimiter terpilih berubah; dash count, pipes/whitespace dan isi sel tetap persis. Satu pilihan satu Atomic Undo; nested/oversized/nonkanonis tidak memperoleh kontrol.
- **Prototipe 6g: Row N Duplicate / Delete** dalam **Table actions…** untuk body tabel root dengan whole physical pipe-leading LF-terminated lines. Header/delimiter dilindungi, raw row disalin/dihapus persis, satu aksi satu Atomic Undo. Duplicate yang melampaui cap lama ditolak; EOF tanpa LF/nonkanonis/nested/fallback tetap tanpa row actions.
- **Prototipe 6h: Cell R:C Edit** dalam **Table actions…** membuka panel draft raw Markdown sel root kanonis/rectangular (row0 header, body mulai1). **Enter/Apply** mengganti payload sel saja; **Cancel/Escape** tidak menyentuh Source. Satu baris, maksimal16KiB byte, pipe harus escaped; outer spacing dan sel lain utuh. Selama draft, command dokumen/file baru dipause. Ini bukan inline WYSIWYG.
- **Prototipe 6i: Previous / Next / Apply & Next** di panel draft sel. Navigasi row-major hanya untuk draft unchanged, tanpa wrap; draft berubah tidak dibuang otomatis. Apply & Next mem-preflight proyeksi baru, commit satu sel lalu memuat neighbor dengan snapshot/offset baru. Cancel hanya membuang draft saat ini, bukan Apply sebelumnya; setiap Apply tetap Atomic terpisah.
- **Prototipe 6j: Insert blank before / after** dalam **Table actions…** menyisipkan satu baris kosong sebelum/sesudah body row kanonis. Jumlah kolom mengikuti header, indent mengikuti anchor; byte lama tetap utuh. Satu Undo per insert; caret Source pada sel pertama baru. Header-only/EOF tanpa LF/target stale/growth cap ditolak. Opsi lama tetap di depan; tidak ada auto-add row pada Next.
- **Prototipe 6k: Column N: Insert before / after** dalam **Table actions…** menyisipkan satu kolom kosong secara atomik pada header, delimiter dan semua body row. Hanya tabel root kanonis rectangular ber-LF; alignment/teks/spacing lama utuh. Satu Undo per kolom; caret Source pada sel header baru. Header-only didukung, ragged/EOF tanpa LF/target stale/cap ditolak. Opsi baru di akhir, tanpa shortcut/implicit creation.
- **Prototipe 6l: Column N: Delete** dalam **Table actions…** menghapus satu kolom root kanonis rectangular (header/delimiter/body) sebagai satu transaksi Undo. Byte/alignment kolom yang bertahan tetap utuh, candidate dipreflight sebelum commit. **Kolom terakhir dilindungi**; deletion tetap boleh pada growth cap. Caret Source menuju payload header neighbor yang bertahan. Ini aksi destruktif eksplisit, bukan auto-delete atau pembuangan draft; opsi lama tetap di depan.
- **Prototipe 6m: Row N: Move up / down** dalam **Table actions…** menukar dua whole raw body lines secara atomik. Isi/indent/tab/ragged width ikut baris asal; header/delimiter tidak bisa dilintasi. Satu Undo per effective swap, caret Source di awal selected row pada posisi baru. Identical raw rows no-op tanpa history/revision tambahan. Opsi move di tail seluruh menu lama, tanpa drag/shortcut/implicit Apply.
- **Prototipe 6n: Column N: Move left / right** dalam **Table actions…** menukar dua kolom bertetangga pada header/delimiter/body sebagai satu transaksi Undo. Alignment dan raw whitespace/markup ikut kolom asal; caret Source menuju payload header selected column di posisi baru. Hanya canonical root rectangular LF, tanpa wrap. Entire raw columns identik no-op; header-only/growth caps tetap didukung. Opsi baru di tail, tanpa drag/shortcut/implicit Apply.
- **Prototipe 6o: Column N: Duplicate** dalam **Table actions…** menyalin header/delimiter/alignment/seluruh body kolom raw ke sebelah kanannya sebagai satu transaksi Undo. Caret Source menuju payload header salinan. Header-only/single-column/empty didukung; growth cap12kolom/128row/512sel tetap. Target canonical rectangular warm tervalidasi, tanpa implicit Apply; opsi append setelah seluruh prefix6n.
- **Prototipe 6p: Apply & Previous** pada panel draft sel menerapkan satu payload secara Atomic lalu membuka sel real sebelumnya secara row-major, termasuk body→header. Target sebelumnya di-resolve dari candidate parser sebelum commit dan distamp Snapshot baru. Sel pertama tidak wrap/Apply otomatis; no-op berpindah tanpa history. Cancel hanya membuang draft saat ini, caret Source tetap di akhir payload yang diterapkan; kontrol lama tetap posisinya.
- **Prototipe 7a: Edit paragraph…** pada paragraf root satu baris ber-LF membuka panel draft raw Markdown (maksimal16KiB, unindented/nonempty/tanpa control chars). Apply/Enter satu Atomic Undo; Cancel/Escape tanpa mutation. Full candidate parser harus mempertahankan complete root paragraph, sehingga conversion heading/list/quote/code/HTML block ditolak. Source caret di akhir payload; raw bytes di luar payload utuh. Belum multiline/EOF atau free inline Preview typing.
- **Prototipe 7b: Edit multiline…** pada paragraf root 2–128 baris nonempty/unindented ber-LF membuka native draft editor terpisah (total16KiB). Enter hanya menambah newline draft; tombol Apply eksplisit melakukan satu Atomic Undo setelah full parser preflight mempertahankan complete paragraph. Cancel tanpa Source mutation; no-op tanpa history. Soft/hard breaks dan byte di luar payload tetap raw, caret Source di akhir baris terakhir yang diterapkan. Panel satu baris7a tidak diubah; belum blank-separated/EOF/inline WYSIWYG.
- **Prototipe 7c: Undo/Redo lokal draft multiline** via Ctrl+Z, Ctrl+Y/Ctrl+Shift+Z saat editor draft fokus, atau tombol Undo draft/Redo draft. Satu edit native efektif satu langkah, termasuk paste/newline/teks invalid; maksimal100record/1MiB raw snapshot. Cancel membuang seluruh riwayat lokal; Apply tetap satu Atomic Undo Source. Restore caret byte logis dengan selection collapsed, belum typing coalescing atau restore scroll/selection anchor. Panel satu baris dan sel belum punya history lokal.
- **Prototipe 7d: Previous multiline / Next multiline** menelusuri paragraf multiline root eligible dalam urutan Source, melewati heading/single-line/nested/code/unsupported. Hanya draft unchanged tanpa error boleh berpindah; draft changed tetap utuh dan Apply tetap eksplisit. Tidak wrap/auto-Apply atau menambah Undo Source. Perpindahan sukses membuka sesi lokal baru dengan riwayat Undo/Redo kosong, termasuk membuang redo sesi lama setelah Undo kembali ke teks asli.
- **Prototipe 7e: Apply & Next multiline** menerapkan satu paragraf lalu membuka root multiline eligible berikutnya, dengan snapshot/range baru dan history lokal kosong. Full candidate parser serta native Source dan next-draft buffers dipreflight sebelum commit. Last boundary/refusal tanpa partial Apply; unchanged hanya pindah tanpa Source edge. Cancel sesi next mempertahankan Apply sebelumnya dan mengembalikan Source caret ke akhir payload yang diterapkan. Belum Apply & Previous atau shortcut Apply.
- Open, Save, Save As, dan ekspor HTML dengan CSS Catppuccin.
- Konfirmasi perubahan belum disimpan sebelum New, Open, dan penutupan jendela.
- Penyimpanan atomik dan pelacakan revisi dokumen untuk hasil operasi asinkron.
- Sidebar dokumen aktif; status bar posisi kursor, jumlah baris, dan word count.
- Iterasi Source 4a–4j: undo/redo terbatas dengan pengelompokan ketikan/hapusan sederhana, Find/Replace literal, Go to Line, inline markup, heading/list/quote, link/code block, continuation list sederhana, serta indent/unindent, duplikasi, hapus dan pindah baris. Suite Cargo/Iced dan smoke GUI X11 virtual sudah lulus; desktop Wayland/Hyprland masih perlu diuji.
- Syntax highlighting Markdown dasar, read-only, dengan palet terang/gelap; berlaku pada Source, Split, dan Zen.
- Zen mode; pada Hyprland juga mengirim perintah toggle maximize.
- Konfigurasi TOML parsial, validasi dengan peringatan, font/wrapping/Tab yang diterapkan pada Source dan Split, serta pilihan tema bawaan Iced.

Preview memakai model block/inline dan rich text Iced: heading, bold/italic/strike, inline code, tautan bergaya read-only, nested list/task/quote, checkbox task kanonis interaktif, code block, rule, serta **tabel GFM native dengan header/alignment dan scroll horizontal**. Warna mengikuti tema. Tabel dibatasi 12 kolom, 128 row termasuk header dan 512 sel; jika melebihi salah satu batas, seluruh source tabel tampil sebagai fallback. HTML/footnote/math tetap source inert; gambar tidak dimuat dan tautan tidak dibuka. Selain checkbox task, level heading root ATX/Setext, alignment/operasi whole body row/insert-hapus-pindah-duplikasi kolom dan panel draft raw sel tabel/paragraf root, teks/blok tetap tidak editable secara bebas. Ini **belum WYSIWYG umum** atau mapping glyph/selection dua arah. Navigasi provenance hanya menuju awal rentang byte blok atau sel, bukan posisi glyph yang diklik. [Fondasi renderer](docs/testing-stage5a.md), [tabel Stage 5b](docs/testing-stage5b.md), [cache Stage 5c](docs/testing-stage5c.md), [navigasi blok Stage 5d](docs/testing-stage5d.md), [navigasi sel Stage 5e](docs/testing-stage5e.md), [prototipe task Stage 6a](docs/testing-stage6a.md), [history task Stage 6b](docs/testing-stage6b.md), [heading ATX Stage 6c](docs/testing-stage6c.md), [heading Setext Stage 6d](docs/testing-stage6d.md), [batch task Stage 6e](docs/testing-stage6e.md), [alignment tabel Stage 6f](docs/testing-stage6f.md), [body row Stage 6g](docs/testing-stage6g.md), [draft sel Stage 6h](docs/testing-stage6h.md), [navigasi sel Stage 6i](docs/testing-stage6i.md), [insert baris Stage 6j](docs/testing-stage6j.md), [insert kolom Stage 6k](docs/testing-stage6k.md), [hapus kolom Stage 6l](docs/testing-stage6l.md), [pindah baris Stage 6m](docs/testing-stage6m.md), [pindah kolom Stage 6n](docs/testing-stage6n.md), [duplikasi kolom Stage 6o](docs/testing-stage6o.md), [Apply & Previous Stage 6p](docs/testing-stage6p.md), [draft paragraf Stage 7a](docs/testing-stage7a.md), [draft multiline Stage 7b](docs/testing-stage7b.md), [history draft Stage 7c](docs/testing-stage7c.md), [navigasi draft Stage 7d](docs/testing-stage7d.md), dan [Apply & Next multiline Stage 7e](docs/testing-stage7e.md). Ekspor HTML tetap independen dan lebih lengkap daripada preview native.

## Build di Arch Linux

Pasang toolchain Rust stable melalui `rustup`, atau gunakan paket `rust` Arch yang cukup baru. Pilih salah satu, jangan memasang keduanya sekaligus.

```sh
sudo pacman -S --needed base-devel rustup pkgconf fontconfig \
  libxkbcommon libxkbcommon-x11 wayland libglvnd libx11 libxcursor libxi libxrandr libxinerama
rustup default stable
rustup component add rustfmt clippy

cargo build --locked
cargo run --locked
```

Untuk build release:

```sh
cargo build --locked --release
./target/release/hylite
```

Build pertama membutuhkan internet untuk dependensi Cargo. Driver grafis yang sesuai dengan GPU juga dibutuhkan saat menjalankan aplikasi. Build CI menggunakan Ubuntu; build dan perilaku runtime di Arch + Hyprland tetap perlu diuji langsung.

### File dialog pada Hyprland

Dialog file `rfd` menggunakan portal desktop pada konfigurasi Linux saat ini. Sediakan backend portal yang sesuai, misalnya:

```sh
sudo pacman -S --needed xdg-desktop-portal xdg-desktop-portal-hyprland \
  xdg-desktop-portal-gtk
```

Backend Hyprland dan GTK memiliki cakupan portal berbeda; pastikan konfigurasi portal sesi memilih backend yang menyediakan `FileChooser`. Jalankan aplikasi dari sesi desktop dengan D-Bus dan environment Wayland/Hyprland yang benar. Hyprland tidak diwajibkan untuk mengedit dokumen; integrasi compositor bersifat opsional.

## Pemeriksaan pengembangan

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --locked --all-targets --all-features
cargo test --locked --all-targets --all-features
```

Workflow `.github/workflows/ci.yml` menjalankan pemeriksaan tersebut pada Linux menggunakan Rust stable. Unit test konfigurasi, state aplikasi, dan operasi file tidak membuka jendela/dialog native dan tidak menulis ke konfigurasi pengguna. Pengujian filesystem memakai direktori sementara. Step GUI terpisah membuka aplikasi nyata pada Xvfb privat, menguji clipboard/keyboard/undo, dan mengunggah screenshot sebagai artifact; lihat [runner GUI X11](docs/testing-gui-x11.md). Checklist pengujian desktop tersedia di [docs/testing-stage2.md](docs/testing-stage2.md), [docs/testing-stage3.md](docs/testing-stage3.md), dan [docs/testing-stage4.md](docs/testing-stage4.md).

`Cargo.lock` disimpan ke Git agar versi dependensi konsisten. Gunakan `--locked` untuk build/lint/test biasa. Pembaruan dependensi dilakukan secara eksplisit melalui `cargo update`, ditinjau, lalu diuji ulang.

### Status verifikasi — 23 Oktober 2026

- Format seluruh sumber diperiksa; `Cargo.lock` tidak berubah dalam Stage 5a–7a.
- **Build native debug, Clippy `-D warnings`, dan 801 tes Cargo lulus** pada Ubuntu 24.04 / Rust stable, dengan `--locked --all-targets --all-features`. Bukti: [CI run 39879922775](https://github.com/Indrawan007/Hylite/actions/runs/39879922775), commit `56865a7`.
- **277 tes core** lulus mandiri melalui `./scripts/test-source-core.sh`, **subset 801 tes Cargo**, bukan tambahan tes unik. Stage 7a menambah 26 tes Rust: 5 planner, 2 history, 7 Document, 3 action/model, 8 controller dan 1 UI adapter.
- **Delapan belas suite GUI X11/Xvfb + tiny-skia lulus: 116 regresi + 9 batch + 9 alignment + 9 row + 14 cell draft + 16 cell navigation + 11 row insertion + 11 column insertion + 11 column deletion + 11 row move + 11 column move + 11 column duplicate + 16 Apply Previous + 15 paragraph draft + 16 paragraph multiline + 24 paragraph history + 19 paragraph navigation + 23 paragraph Apply Next**, total **352 pemeriksaan** (321 clipboard: 263 Source + 58 draft input; 12 title konfirmasi; 19 exit native normal). Gate lama utuh, masing-masing private Xvfb terpisah dan alarm180s. Dirty draft refusal, explicit Apply/Apply & Next/Previous dengan offset Unicode baru, insert baris/kolom kosong before/after, atomic Delete/Duplicate kolom/Move body row/Move kolom dengan alignment, draft paragraf root/multiline dengan parser refusal, history lokal serta fokus/caret Split tanpa Source refocus teruji. Screenshot bukan pixel/glyph oracle. [Bukti GUI](docs/testing-gui-x11.md) dan [Apply & Next Stage 7e](docs/testing-stage7e.md).
- **51 tes Python helper** (4 reporter + 47 runner GUI) lulus. Tidak mengubah koordinat/byte oracle/refocus gate lama. [Riwayat native](docs/verification-native.md) mencatat koreksi oracle 6f serta startup exit101 lama yang belum terdiagnosis.
- Preview mempunyai checkbox (single/batch), level heading root ATX/Setext, alignment delimiter, whole raw body row/move baris-kolom/insert baris-kolom kosong/hapus-duplikasi kolom dan panel draft raw teks sel root dengan navigasi/Apply & Next/Previous serta single-line root paragraph Apply/Cancel terbatas. Undo/Redo hanya edge eligible di puncak. Ini **belum inline WYSIWYG**: mengetik bebas pada teks Preview, rich selection formatting, multi-cell selection, glyph/selection mapping dan synchronized scrolling belum ada. Wayland/Hyprland, portal file dialog, IPC, IME/RTL, layout visual desktop target dan renderer GPU masih perlu pengujian langsung; kelulusan X11/unit test tidak menggantikannya.

## Konfigurasi

Pada Linux, konfigurasi biasanya berada di:

```text
$XDG_CONFIG_HOME/hylite/config.toml
# atau ~/.config/hylite/config.toml jika XDG_CONFIG_HOME tidak disetel
```

Aplikasi mencoba membuat konfigurasi default saat file belum ada. `config/default.toml` adalah contoh, bukan file yang dimuat langsung.

Konfigurasi dimuat saat startup, belum mendukung live reload. Field maupun section boleh dihilangkan; nilainya diambil dari default. Contoh minimal:

```toml
[editor]
font_size = 18.0
word_wrap = false
default_mode = "split"

[appearance]
theme = "nord"
```

Opsi yang saat ini diterapkan:

| Section | Opsi | Default / batas |
| --- | --- | --- |
| `editor` | `font_family` | `monospace`; 1–128 karakter tanpa karakter kontrol, whitespace tepi dipangkas |
| `editor` | `font_size` | `16.0`; nilai finite dalam 8–72 |
| `editor` | `tab_size` | `4`; 1–16, spasi menuju tab stop berikutnya |
| `editor` | `syntax_highlighting` | `true`; warna Markdown dasar, `false` untuk source plain |
| `editor` | `word_wrap` | `true`; wrapping kata/glyph, atau tanpa wrapping jika `false` |
| `editor` | `default_mode` | `source`, `preview`, atau `split` |
| `appearance` | `theme` | `catppuccin-mocha`; pilihan di bawah |
| `appearance` | `content_width` | `820`; 320–2400, lebar maksimum preview/Zen |
| `appearance` | `sidebar_width` | `220`; 120–600 |

Font, ukuran font, wrapping, dan binding Tab dipakai oleh widget source yang sama pada Source dan Split. `font_family` menerima `monospace`, `sans-serif`, `serif`, atau nama font yang terpasang, misalnya `JetBrains Mono Nerd Font`. Font tidak dibundel dan fallback font bergantung pada text engine Iced. Nama font custom ditahan sepanjang proses karena API font Iced 0.13 membutuhkan nama dengan lifetime static; alokasi dilakukan sekali saat startup, bukan saat render.

Tab tanpa seleksi menyisipkan spasi menuju tab stop sebagai satu edit. Dengan seleksi, Tab mengindentasi seluruh baris tersentuh; Shift+Tab menghapus satu tingkat indentasi. Tab literal yang sudah ada/pasted tidak dikonversi; lebar visualnya masih ditentukan text engine Iced. Kolom dihitung dalam Unicode scalar, bukan lebar visual/grapheme.

Tema yang diterima: `catppuccin-mocha`, `catppuccin-latte`, `catppuccin-frappe`, `catppuccin-macchiato`, `nord`, `dracula`, `gruvbox-dark`, `tokyo-night`, `dark`, dan `light`. Preview Stage 5a mengikuti palet tema aktif; sebagian warna UI lain masih hardcoded sehingga konsistensi tema terang seluruh aplikasi belum lengkap.

### Validasi dan migrasi

- Nilai numerik/font yang di luar batas dikembalikan **per field** ke default; opsi valid lain dipertahankan.
- Kesalahan sintaks, tipe field, key tidak dikenal, atau enum mode/tema tidak dikenal menyebabkan **seluruh konfigurasi** ditolak dan default dipakai.
- Semua peringatan menyebut lokasi file, ditampilkan dalam banner yang bisa ditutup (termasuk di Zen), serta ditulis ke stderr. File konfigurasi yang salah tidak ditimpa.
- File default baru dibuat secara privat (`0600`) menggunakan `create_new`, sehingga tidak menimpa file yang muncul bersamaan atau symlink putus. Kegagalan baca/tulis dilaporkan. Jika direktori konfigurasi pengguna tidak dapat ditemukan, aplikasi memakai default tanpa menulis ke working directory.
- `auto_save` masih diterima untuk kompatibilitas, tetapi `true` dinonaktifkan dengan peringatan sampai fitur recovery/konflik matang.
- **Hapus `appearance.opacity` dari konfigurasi lama.** Opsi ini sebelumnya tidak diterapkan, kini ditolak sebagai key tidak dikenal. Atur opacity melalui Hyprland. `config/default.toml` sekarang selaras dengan default aplikasi.

## Shortcut

| Shortcut | Aksi |
| --- | --- |
| `Ctrl+O` | Buka file |
| `Ctrl+S` | Simpan |
| `Ctrl+Shift+S` | Save As |
| `Ctrl+N` | Dokumen baru |
| `Alt+S` / `Alt+D` | Save & Continue / **Discard & Continue**, hanya saat konfirmasi aktif |
| `Ctrl+B` | Toggle sidebar, bukan bold |
| `Ctrl+Shift+B` / `Ctrl+I` / `Ctrl+E` | Bold / italic / inline code, saat Source fokus |
| `Ctrl+1` … `Ctrl+6` | Toggle/set heading 1–6, saat Source fokus |
| `Ctrl+Shift+L` / `Ctrl+Shift+O` / `Ctrl+Shift+T` | Bullet / numbered / task list, saat Source fokus |
| `Ctrl+K` / `Ctrl+Shift+C` | Sisipkan link / fenced code block, saat Source fokus |
| `Ctrl+Shift+D` / `Ctrl+Shift+K` | Duplicate Lines / Delete Lines, saat Source fokus |
| `Alt+↑` / `Alt+↓` | Pindahkan baris/blok ke atas / bawah, saat Source fokus |
| `Ctrl+Z` / `Ctrl+Shift+Z` atau `Ctrl+Y` | Undo / redo, saat Source fokus |
| `Ctrl+F` / `Ctrl+H` | Buka Find / Replace |
| `Ctrl+G` | Go to Line: baris atau baris:kolom |
| `F3` / `Shift+F3` | Match berikutnya / sebelumnya, berputar |
| `Tab` / `Shift+Tab` | Spasi atau indent / unindent |
| `Ctrl+Shift+M` | Siklus Source → Preview → Split |
| `Ctrl+Shift+E` | Ekspor HTML |
| `F11` | Toggle Zen |
| `Esc` | Batalkan konfirmasi → tutup error → tutup Go to Line / Find → keluar Zen |

Saat konfirmasi perubahan belum disimpan aktif, title menampilkan `[confirm New/Open/Close]` dan tombol menjelaskan shortcut kontekstual. **Alt+D membuang perubahan** setelah pilihan itu dibuat; Escape membatalkan dan memulihkan fokus input sebelumnya. Ctrl+Alt (AltGr), Shift+Alt dan logo tidak dipetakan ke akselerator konfirmasi. Native close client X11 sudah diuji; Save & Continue melalui portal file dan perilaku compositor Wayland/Hyprland belum diuji end-to-end.

## Source editor (iterasi 4a–4j)

Find bersifat literal dan case-sensitive, tanpa regex atau mode whole-word. Panel menampilkan jumlah, navigasi berputar, serta konteks match; kursor diarahkan ke match, **bukan highlight/seleksi native**. Gunakan Next dulu sebelum Replace satu hasil. Replace All memakai match asli satu kali, dapat menghapus dengan replacement kosong, dan menjadi satu transaksi undo.

Undo/redo menyimpan teks dan caret, **bukan seleksi**, dengan batas gabungan **200 entri / 8 MiB byte teks yang ditahan**. Ketikan insert dan penghapusan single-scalar berurutan dapat dikelompokkan sesuai aturan 4d/4g di bawah; paste/formatting/penggantian seleksi tetap atomic. History direset saat New/Open; edit sesudah undo membuang cabang redo. Jika state melebihi budget, perubahan itu mungkin tidak dapat di-undo/redo. Batas tersebut bukan batas memori seluruh aplikasi; ada salinan sementara dan buffer native. Snapshot penuh, rekonstruksi caret, pencarian, dan word count masih O(ukuran dokumen), belum cocok untuk file besar.

Bold/italic/code membungkus seleksi source, atau menyisipkan placeholder `text` bila seleksi kosong. Pilih seluruh marker yang telah dibungkus untuk toggle. Inline code menolak seleksi multiline/backtick internal yang tidak didukung; ini belum formatter AST. Formatting dan indentasi mengakhiri seleksi. Undo mengembalikan teks/caret, tidak mengembalikan anchor seleksi. Menu **Blocks…** menyediakan heading 1–6, bullet/numbered/task list, dan block quote. Perintah bekerja pada baris logis yang tersentuh, tanpa memasukkan baris berikutnya bila seleksi berakhir pada kolom nol. Prefix yang sama pada semua baris nonblank ditoggle lepas; seleksi campuran dinormalisasi. Indentasi dan whitespace body dipertahankan, numbered list baru dihitung dari 1 melewati baris kosong. Task yang sudah dicentang dipertahankan dalam seleksi campuran; toggle lepas/konversi ke list biasa menghapus checkbox. Baris blank dilewati; bila seluruh seleksi blank, hanya baris pertama diberi prefix.

Link membungkus label satu baris, meng-escape bracket/backslash, dan menaruh caret setelah `https://`, sebelum `)`, agar URL dilengkapi. Link kosong memakai label `text`; ini insertion, bukan editor URL/toggle link. Code block membungkus seluruh baris tersentuh menggunakan backtick fence lebih panjang dari run backtick mana pun di body. Caret tetap di dalam body; tidak ada toggle unwrap atau deteksi bahasa.

Semua command tersebut menjadi satu transaksi undo. Formatter ini **leksikal, bukan AST-aware**: tidak mengenali konteks code fence/container di luar seleksi, tidak me-reflow continuation line list atau merombak indentasi Markdown. Periksa hasil source/preview; indentasi yang dipertahankan bisa berarti code block dalam Markdown. Continuation list dan grouping deletion sederhana dijelaskan di bawah; grouping multi-scalar dan UX seleksi lebih matang masih pekerjaan berikutnya.

### Hardening workflow core (4j)

Iterasi ini tidak menambah fitur UI atau mengubah perilaku runtime. Empat regresi gabungan menjalankan modul produksi inti bersama-sama: History, transaksi Source, Enter list, navigasi dan pencarian. **64 seed tetap × 512 langkah = 32.768 langkah workflow**, dibandingkan dengan oracle history berbasis `Vec<char>`/cursor scalar dan stack terpisah. Termasuk insert/hapus, paste/replacement, operasi baris, Find/Replace All, indent, navigasi, barrier capture save, no-op, undo/redo serta clock boundary/mundur. Seluruh checkpoint undo/redo yang tersisa juga di-drain dan dibandingkan, bukan hanya teks akhir.

**16 kombinasi budget × 512 langkah = 8.192 langkah tambahan** memeriksa batas gabungan entri/byte, caret UTF-8 dan base grup yang masih tersedia. Dua tes lain memeriksa checkpoint save lintas transaksi dan 192 variasi delta kandidat yang tidak valid/no-op. Tidak ada dependency, sleep, file data hasil generate, atau akses konfigurasi pengguna baru; seed dan clock tes tetap agar kegagalan bisa direproduksi.

Ini **test rig core, bukan `Document` atau controller Iced**. Oracle memeriksa history/scalar state; untuk transaksi formatter, planner produksi memberi transaksi yang diterapkan ke kedua representasi (semantik planner diuji terpisah). Budget test memeriksa invariant, bukan oracle kebijakan trim independen. Snapshot/revisi dalam rig hanyalah input pengujian cache. Lulusnya workflow tidak membuktikan caret/selection/viewport native, guard aplikasi, parser code context, serialisasi/save ke disk, atau GUI/Hyprland. Build dan suite Cargo terpisah kini sudah lulus (lihat status verifikasi); pengujian desktop dan pekerjaan Source yang tersisa tetap diperlukan sebelum Tahap 4 dinyatakan selesai.

Jalankan hanya empat regresi workflow (seluruh harness tetap dikompilasi):

```sh
./scripts/test-source-core.sh workflow_tests --nocapture
```

### Operasi baris (4h)

**Ctrl+Shift+D — Duplicate Lines**, **Ctrl+Shift+K — Delete Lines**; tombol yang sama tersedia di toolbar. Shortcut hanya berlaku saat editor Source fokus, bukan di input Find/Go to Line; **Ctrl+K tetap Insert Link**. Tanpa seleksi, operasi memakai baris logis saat ini; dengan seleksi, seluruh baris tersentuh. Seleksi berakhir di kolom nol baris berikutnya tidak memasukkan baris tersebut.

Duplikasi menyalin blok **secara literal ke bawahnya**: indentasi, checkbox, nomor dan markup tidak diubah atau dihitung ulang. Tanpa seleksi, caret berada pada kolom byte UTF-8 yang sama di salinan; dengan seleksi, caret di awal blok salinan. Baris EOF nonempty tanpa LF tetap tanpa LF terminal setelah duplikasi; duplikasi baris kosong (termasuk dokumen kosong) menambah satu LF.

Delete Lines bukan Cut: clipboard tidak disentuh. Baris nonfinal dihapus bersama LF sesudahnya, sehingga caret menuju awal baris berikutnya. Untuk blok di EOF tanpa LF sesudahnya, LF sebelumnya ikut dihapus dan caret berada di akhir baris sebelumnya; menghapus baris kosong terakhir berarti melepas satu LF. Menghapus seluruh isi menyisakan dokumen kosong; Delete Lines pada dokumen kosong adalah no-op.

Setiap aksi efektif menjadi **satu transaksi undo**, terpisah dari ketikan/hapusan di sekitarnya; Find di-refresh dan guard prompt/Open/Preview tetap berlaku. Save/ekspor biasa mengizinkan edit. Undo mengembalikan teks/caret, **bukan anchor seleksi**. Ini operasi raw Source, termasuk di code fence: tidak menjaga struktur AST atau keseimbangan Markdown. Scan/copy/rekonstruksi masih O(ukuran dokumen), dengan budget history yang sama.

Jalur New/Open mengisi buffer dengan Paste yang dilindungi sebelum tracking history, bukan `with_text`, agar baris kosong terakhir tidak terbuang oleh initializer Iced 0.13. Proteksi satu spasi sementara lalu Backspace juga menjaga tab di akhir string yang dipangkas Paste mentah cosmic-text 0.12.1. Jalur yang sama digunakan untuk paste pengguna dan rekonstruksi caret; aksi internal tidak menjadi entri undo terpisah. Tidak membuat revisi/dirty/history saat load, dan caret awal tetap di awal dokumen. Regresi loader/EOF/tab/undo dan integrasi operasi lulus di suite Cargo. Paste tab/LF dan undo/redo juga lulus pada smoke GUI X11; operasi baris melalui GUI dan desktop Wayland/Hyprland tetap belum diuji.

### Pindahkan baris/blok (4i)

**Alt+↑ / Alt+↓** atau tombol **Move Up / Move Down** menukar baris saat ini/blok seleksi dengan tepat satu baris di sebelahnya. Shortcut hanya berlaku untuk **Alt tanpa Ctrl/Shift/logo** pada editor Source focused; Ctrl+Alt/AltGr tidak menjalankan operasi ini. Binding global aplikasi tidak mengambil shortcut tersebut. Jika compositor sudah memakainya, gunakan tombol toolbar atau sesuaikan binding compositor.

Baris/blok dipindahkan literal, urutan di dalam blok tidak berubah, tidak ada renumber list/reset checkbox/AST balancing. Endpoint seleksi di kolom nol tidak memasukkan baris berikutnya. Tanpa seleksi, caret mengikuti baris pada byte-column UTF-8 yang sama; dengan seleksi, caret berada di awal blok pindahan dan **seleksi dikolapskan**. Karena anchor belum dipertahankan, menekan shortcut lagi setelah memindah blok akan bekerja pada baris caret, bukan otomatis pada seluruh blok sebelumnya.

Keberadaan LF terminal, jumlah LF, dan seluruh isi baris dipertahankan. Baris kosong final setelah LF terakhir tidak menjadi target/baris yang bisa dipindah. Blank line lain tetap bisa dipindah, kecuali pertukaran di EOF dokumen tanpa LF yang akan mengubah status LF akhir; kasus itu ditolak. Batas atas/bawah dan pertukaran identik juga no-op: caret/checkpoint/revisi/redo tidak berubah (anchor seleksi tetap bisa dikolapskan, sama seperti formatter lain).

Setiap perpindahan efektif satu transaksi undo, terpisah dari grup ketikan/hapusan. Guard prompt/Open/Preview, refresh Find, checkpoint save dan budget history tetap sama dengan 4h. Undo memulihkan teks/caret, bukan seleksi. Ini memakai baris logis, bukan soft-wrap visual; scan/copy/rekonstruksi masih O(ukuran dokumen). Tes core mencakup permutasi baris independen serta round-trip caret Unicode; regresi model/controller/binding lulus di Cargo. Fokus/viewport dan perilaku compositor desktop **belum diverifikasi**.

### Go to Line (4f)

**Ctrl+G** atau tombol **Go to Line** membuka panel dengan posisi saat ini. Masukkan `12` untuk awal baris 12, atau `12:5` untuk kolom 5. Koordinat dimulai dari 1 dan mengacu pada **baris logis serta Unicode scalar**, sama seperti status bar—bukan baris soft-wrap, byte UTF-8, grapheme, atau lebar visual. Tab, combining mark, dan setiap scalar emoji dihitung sendiri. Kolom setelah karakter terakhir valid; dokumen kosong maupun baris kosong setelah LF terakhir mempunyai kolom 1.

Nol, tanda plus/minus, digit non-ASCII, format salah, integer overflow, alamat >64 byte, dan lokasi di luar dokumen ditolak dengan error pada panel; tidak ada clamping diam-diam. Whitespace di luar alamat boleh, tetapi tidak di tengah `baris:kolom`. Alamat diselesaikan terhadap buffer saat tombol Go/Enter ditekan. Input invalid tidak memindahkan caret atau mengubah seleksi.

Navigasi berhasil menutup panel dan meminta fokus Source; Escape/Close membatalkan panel sebelum keluar Zen. Go to Line dan Find tidak ditampilkan bersamaan; berpindah ke Preview menutup panel, sedangkan Ctrl+G dari Preview membuka Source. Navigasi tidak mengedit teks, dirty, revisi, checkpoint, atau stack undo/redo, tetapi menutup burst ketikan dan mengakhiri seleksi saat caret berhasil dipindah. Target Find aktif dibersihkan agar Replace tidak memakai hasil lama. Prompt/Open memblokir navigasi, save/ekspor biasa tidak; New/Open sukses mereset panel agar submit lama tidak berlaku pada dokumen pengganti.

Offset dihitung UTF-8 lalu diterapkan lewat rekonstruksi caret yang sama dengan Find. Masih ada salinan/scan O(ukuran dokumen), bukan indeks baris incremental. Caret scalar Unicode dan fokus keyboard setelah submit/Escape sudah dibuktikan pada fixture smoke GUI X11. Visibilitas caret/scrolling/IME/RTL dan perilaku desktop Wayland/Hyprland **belum diverifikasi**. Gutter nomor baris tetap ditunda: `Content` Iced 0.13 tidak membuka layout baris visual/scroll kepada kode aplikasi; kolom nomor terpisah tanpa data itu tidak menjamin keselarasan saat soft-wrap.

### Enter pada list (4e)

Enter tanpa modifier pada Source/Split/Zen melanjutkan list sederhana dengan **0–3 spasi awal**, tanpa quote prefix. Marker `-`/`+`/`*`, delimiter nomor `.`/`)`, indentasi, dan separator dipertahankan. Nomor bertambah satu (termasuk zero padding); task baru selalu `[ ]`, bukan menyalin centang. Memecah di tengah body mempertahankan suffix setelah caret. Pada item dengan body kosong atau hanya spasi/tab, Enter melepas marker/checkbox **di tempat**, menyisakan indentasi tanpa menambah LF. Satu aksi menjadi satu langkah undo terpisah dari ketikan di sekitarnya, dalam batas budget history.

**Shift+Enter** memakai Enter native tanpa continuation. Seleksi aktif, indentasi ≥4 spasi/tab, quoted list, caret di dalam prefix, nomor nonempty yang melampaui 9 digit, thematic rule, atau baris >16 KiB juga memakai fallback native. Tidak ada auto-indent paragraf umum, penghitungan ulang sibling, outdent bertingkat, atau parsing struktur list penuh. Sebagian nested list berindentasi pendek cocok secara leksikal, tetapi bukan dukungan nested-list struktural.

Guard fence ringan bersifat konservatif dan dapat menolak continuation lebih lama pada konteks ambigu. Kandidat yang lolos juga diperiksa dengan rentang code block dari parser Markdown yang sudah dipakai aplikasi, sehingga container/fence kompleks tidak bergantung pada warna syntax. Pemeriksaan parser dilakukan pada kandidat Enter, bukan setiap ketikan; tetap O(ukuran dokumen), bukan parser incremental. Tes integrasi parser/model/controller sudah lulus di suite Cargo; perilaku Enter melalui desktop target tetap perlu diuji.

Untuk menonaktifkan lalu restart (default `true`, termasuk config lama tanpa field ini):

```toml
[editor]
auto_continue_lists = false
```

### Pengelompokan hapusan (4g)

Backspace berurutan atau Delete berurutan tanpa seleksi dapat menjadi **satu langkah undo**. History memeriksa delta buffer sebenarnya: harus tepat satu Unicode scalar di sebelah caret lama, prefix/suffix tetap sama, dan caret hasil tepat. Backspace dan Delete mempunyai grup terpisah; tidak digabung dengan insert. Grup teks/whitespace juga terpisah, mengikuti kebijakan 4d (underscore dianggap teks; tanda baca ASCII lain dan control, termasuk tab/newline, atomic).

Batas yang sama berlaku: idle **<750 ms**, durasi burst **<5 detik**, maksimum **64 scalar** per grup. Navigasi, seleksi, Go to Line/Find, command UI, kehilangan fokus, undo/redo, dan capture save menutup grup. Menghapus di BOF/EOF tanpa perubahan menutup grup, tetapi tidak membuat dirty/revisi/history baru atau membuang redo. Setiap penghapusan efektif tetap mendapat revisi/state ID baru, walaupun digabung dalam undo.

Ini **bukan penggantian algoritme hapus native atau segmentasi grapheme penuh**. Jika satu aksi native menghapus beberapa scalar sekaligus (word/grapheme kompleks), menggabungkan newline, menghapus seleksi, atau menghasilkan delta/caret yang tidak cocok, aksi itu tetap atomic. Tidak ada shortcut word-delete baru maupun pengaturan kebijakan grouping. Caret dipulihkan saat undo/redo; anchor seleksi tetap tidak disimpan. Batas 200 langkah/8 MiB dan salinan buffer O(n) tetap berlaku. Integrasi penghapusan Iced, IME/RTL dan GUI masih memerlukan verifikasi desktop.

### Pengelompokan ketikan (4d)

Insert satu karakter tanpa seleksi digabung hanya bila delta buffer UTF-8, caret, dan state sebelumnya benar-benar tersambung. Grup teks non-whitespace (termasuk underscore dan Unicode) dan grup whitespace terpisah; tanda baca ASCII selain underscore, control/newline, Paste/IME commit sebagai paste, Backspace/Delete, serta formatting tidak digabung dengan insert. Ini bukan segmentasi kata linguistik atau grapheme-aware; batas dihitung dalam Unicode scalar.

Jeda **≥750 ms**, durasi grup **≥5 detik**, atau karakter ke-**65** memulai langkah baru. Navigasi/seleksi, rekonstruksi caret Find, undo/redo, command UI, dan kehilangan fokus jendela menutup grup. Kebijakan masih tetap, belum dapat dikonfigurasi. Waktu memakai clock monotonik saat aksi diproses, bukan timestamp hardware keyboard.

Save menutup grup **sebelum** mengambil snapshot untuk worker, agar undo bisa kembali ke checkpoint yang benar-benar ditulis, selama batas history belum memangkasnya. Acknowledgment save valid juga menutup grup; hasil stale/invalid tidak boleh memotong grup. Setiap edit efektif tetap mengalokasikan revisi/state ID baru—coalescing hanya mengurangi snapshot undo yang ditahan. No-op tidak membuat entri atau menghapus redo. Salinan teks sementara per aksi masih O(ukuran dokumen); ini belum optimasi file besar.

### Syntax highlighting dasar (4c)

Lexer read-only memberi warna pada heading ATX, prefix list/quote/task, thematic rule, emphasis/strong, inline code, inline link/image, serta fenced code tingkat atas (backtick/tilde). Isi fence diberi satu warna code, **bukan syntax bahasa pemrograman**. Heading diberi warna satu baris; tidak ada font/ukuran/weight tambahan. Palet dasar dipilih berdasarkan tema terang/gelap, belum tema warna per-token yang dapat dikustomisasi.

Ini lexer leksikal, bukan parser CommonMark lengkap: nested container fence, Setext heading, reference link, HTML, tabel, dan emphasis bersarang/flanking penuh belum ditangani. Highlighting tidak mengedit teks atau menjadi langkah undo; warna syntax juga berbeda dari highlight hasil Find (yang tetap belum native).

Untuk menonaktifkan, edit konfigurasi lalu restart:

```toml
[editor]
syntax_highlighting = false
```

Pekerjaan lookahead inline dibatasi 4 × byte baris. Baris lebih dari **16 KiB** tampil plain, namun konteks fence tetap dilacak. Budget habis dapat menyisakan suffix plain; tidak ada pemotongan teks. Cache menyimpan state fence per baris yang diproses, bukan teks, dengan memori O(jumlah baris/kapasitas yang pernah diproses sejak reset). Ini bukan jaminan RAM/latensi untuk file besar. Setelah rekonstruksi buffer (Find/format/undo/redo), cache direset memakai identitas render terpisah dari checkpoint dirty. Integrasi visual dan pengaruh pada viewport native tetap perlu pengujian desktop.

Lihat [checklist Tahap 4](docs/testing-stage4.md) sebelum mencoba pada desktop.

## Keamanan dokumen (Tahap 2)

- Save As memperbarui path aktif hanya setelah penulisan berhasil. Save selanjutnya memakai path tersebut tanpa dialog baru.
- New, Open, dan permintaan tutup jendela pada dokumen berubah menampilkan **Save & Continue / Discard & Continue / Cancel**. Save yang dibatalkan/gagal tidak melanjutkan aksi tersebut.
- Saat konfirmasi aktif, editing diblokir. Saat membuka file, editing juga diblokir hingga hasil diterima. Jika Open gagal/dibatalkan setelah Discard, dokumen lama tetap tersedia beserta penanda perubahannya.
- Hanya satu operasi file berjalan sekaligus. New/Open/Close dan operasi file lain harus menunggu operasi aktif selesai; permintaan tersebut tidak diantrekan otomatis. Ulangi aksi setelah selesai.
- Editing tetap diperbolehkan selama save biasa atau ekspor. Save mencatat checkpoint konten yang benar-benar ditulis; state lain tetap dirty. Undo/redo bisa kembali ke checkpoint tersimpan tanpa memakai ulang nomor revisi operasi.
- Hasil operasi memiliki ID operasi, ID dokumen, revisi monotonik, dan ID checkpoint konten. Hasil yang kedaluwarsa tidak boleh mengubah dokumen pengganti.
- Ekspor tidak mengubah path maupun status tersimpan Markdown. Ekspor melalui dialog menolak tujuan yang merupakan file sumber aktif atau alias symlink/hard-link-nya.
- Kegagalan file menampilkan penyebab, termasuk dalam Zen mode; pembatalan dialog bukan error.

Save Markdown dan ekspor HTML menggunakan file sementara pada direktori tujuan, sinkronisasi file, rename atomik, lalu sinkronisasi direktori. Kegagalan sebelum rename tidak memotong file asli. Jika sinkronisasi direktori gagal setelah rename, aplikasi melaporkan error secara konservatif walaupun isi file mungkin sudah berubah; periksa file sebelum melanjutkan.

Permission mode file lama dipertahankan, sedangkan file baru menggunakan mode privat `0600`. Symlink yang valid ditulis melalui targetnya tanpa mengganti link itu sendiri. Symlink putus, hard-linked file, direktori, dan file read-only ditolak dengan pesan error. Rename membutuhkan izin tulis direktori tujuan. ACL, extended attributes, serta ownership file lama **belum disalin** ke inode pengganti; jangan memakai atomic save ini pada file yang bergantung pada metadata khusus tersebut.

## Batasan penting

- Fitur Tahap 2–3 dan iterasi 4a–4j sudah diimplementasikan; build debug dan suite Cargo lulus di CI Linux. Smoke GUI X11 virtual lulus, tetapi pengujian portal, Arch/Hyprland, IME/RTL, GPU, build release dan packaging belum dilakukan. Tetap gunakan salinan dokumen percobaan.
- Belum ada autosave, recovery crash, workspace, tab dokumen, atau WYSIWYG. Konfirmasi tidak dapat melindungi tulisan dari force-kill atau crash.
- Belum ada deteksi konflik terhadap perubahan dari proses/aplikasi lain; atomic rename bukan mekanisme penguncian lintas proses dan tidak menghilangkan race filesystem.
- Ekspor **tidak menyaring raw HTML**, termasuk script. Jangan membuka hasil ekspor dari Markdown tidak tepercaya di browser tanpa pemeriksaan/sanitasi.
- IPC Hyprland masih sinkron dan tanpa timeout; error belum ditampilkan. Toggle maximize bergantung pada jendela aktif.
- Path dari argumen command line belum diproses.

## Struktur kode

```text
src/
├── main.rs             # Entry point Iced
├── app.rs              # State UI, routing update, dan layout
├── app/
│   ├── file_actions.rs # Orkestrasi New/Open/Save/Export/Close
│   ├── editing.rs      # Guard Source, undo/format/search/replace
│   ├── navigation.rs   # Go to Line, guard, dan koordinasi fokus panel
│   └── tests.rs        # Regresi state aplikasi tanpa menjalankan dialog
├── commands.rs         # Message, aksi tertunda, dan shortcut aplikasi
├── config.rs           # Default, deserialisasi, validasi, dan warning TOML
├── editor.rs           # Enum mode/tema, font, binding native Source
├── search.rs           # Pencarian literal berbasis snapshot dokumen
├── navigation.rs       # Alamat 1-based, offset UTF-8, state panel Go to Line
├── navigation_tests.rs # Input invalid/overflow dan matriks Unicode/EOF
├── preview_navigation.rs # Target provenance blok/sel dan validasi UTF-8
├── preview_task.rs     # Planner one-byte task toggle, snapshot/token guards
├── preview_table_edit.rs # Alignment delimiter root, snapshot/token guards
├── preview_table_row.rs # Duplikasi/hapus whole raw body line tabel root
├── preview_cell.rs     # Planner payload sel, single-line/escape/cap dan target auth
├── text_edit.rs        # Transformasi UTF-8 inline/indent dan rentang baris
├── line_edit.rs        # Duplikasi/hapus/pindah baris logis, transaksi UTF-8
├── line_edit_tests.rs  # Regresi EOF/seleksi dan matriks rentang
├── block_edit.rs       # Heading/list/quote, label link, dan code fence
├── block_edit_tests.rs # Regresi formatter dan matriks batas UTF-8
├── continuation.rs     # Planner Enter list sederhana dan scanner fence
├── continuation_tests.rs # Regresi Enter/fallback dan batas UTF-8
├── syntax.rs           # Lexer read-only dan cache konteks fence
├── syntax_tests.rs     # Rentang UTF-8, batas kerja, replay incremental
├── highlighting.rs     # Adapter Highlighter Iced dan pemetaan warna
├── document/
│   ├── state.rs        # Buffer native, path, revisi, checkpoint dirty
│   ├── history.rs      # History terbatas dan grouping insert/deletion tervalidasi
│   ├── code_context.rs # Guard rentang code block parser sebelum Enter list
│   ├── typing_tests.rs # Kebijakan insert/waktu/Unicode/save boundary tanpa GUI
│   ├── deletion_tests.rs # Delta hapusan/arah/budget tanpa GUI
│   ├── workflow_tests.rs # Oracle scalar, seeded workflow dan invariant budget
│   ├── state_tests.rs  # Regresi model yang membutuhkan dependensi Iced
│   ├── preview_cache_tests.rs # Invalidasi/reuse model lintas revisi/navigation/save
│   ├── snapshot.rs     # Identitas revisi tanpa dependensi UI
│   ├── preview_model.rs # Proyeksi block/inline owned dan provenance UTF-8
│   ├── preview_model_tests.rs # Struktur, fallback, code whitespace, nesting
│   ├── preview_table.rs # Proyeksi tabel GFM, provenance dan batas eager widget
│   ├── preview_table_view.rs # Grid Iced, alignment, header dan scroll horizontal
│   └── preview.rs      # Rich text/widget Iced dan validated Preview edit requests
├── operations.rs       # Serialisasi operasi dan penolakan hasil kedaluwarsa
├── export/mod.rs       # Konversi/ekspor HTML
├── file_io.rs          # Dialog native dan worker I/O blocking
├── files.rs            # Penulisan atomik dan proteksi tujuan ekspor
├── hyprland.rs         # IPC compositor
├── welcome.md          # Dokumen awal
└── ui/                 # Toolbar, sidebar, status, konfirmasi, notifikasi
```

Model dokumen masih memakai `text_editor::Content` milik Iced sebagai satu-satunya buffer teks. Model preview tetap proyeksi immutable. Checkbox 6a mengirim transaksi ke buffer Source lalu diparse ulang; bukan mutasi/serialisasi AST. Buffer belum digantikan editable AST dan WYSIWYG umum belum ada. Detail invariant dan batas antar modul dijelaskan di [docs/architecture.md](docs/architecture.md).

## Urutan pengembangan

1. Fondasi build, test, CI, dan dokumentasi — gate native Linux lulus; release/packaging terpisah.
2. Keamanan dokumen dan perbaikan save/export — tes model/filesystem lulus; portal dan failure runtime masih perlu diuji.
3. Pemisahan model dokumen, operasi file, dan konfigurasi — teruji dalam suite Cargo; desktop target belum diverifikasi.
4. Source editor untuk penggunaan harian — iterasi 4a–4j tersedia; integrasi desktop dan fitur Source lanjutan belum selesai.
5. Renderer Markdown native — 5a read-only, 5b tabel, 5c cache, 5d navigasi blok dan 5e navigasi sel tersedia; cakupan Markdown, mapping dan optimasi lanjutan belum lengkap.
6. Prototipe WYSIWYG dan pemetaan posisi sumber — 6a checkbox, 6b history terbatas, 6c level root ATX, 6d level root Setext, 6e batch task dan 6f alignment kolom dan 6g duplikasi/hapus body row dan 6h panel draft teks sel serta 6i navigasi draft/Apply & Next serta 6j insert baris kosong dan 6k insert kolom kosong serta 6l hapus kolom dan 6m pindah body row serta 6n pindah kolom tabel root terbatas tersedia; editing umum serta glyph/selection mapping belum tersedia.
7. Pengalaman editing ala MarkText.
8. Workspace, recovery, dan integrasi Hyprland yang tangguh.
9. Optimasi, packaging Arch, dan persiapan rilis.

Manifest menyatakan lisensi MIT; penambahan berkas lisensi dan pemeriksaan atribusi kode/aset tetap diperlukan sebelum distribusi.
