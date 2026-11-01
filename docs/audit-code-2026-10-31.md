# Audit Kode Hylite terhadap `AGENT.md`

**Tanggal:** 31 Oktober 2026 · **Branch:** `arena/5c3a966e-hylite` · **Basis commit:** `98a419d` (kode terakhir: `873aba2`)
**Cakupan:** seluruh repositori — 113 file Rust (≈46.900 baris `src/` + `tools/`), 6.191 baris skrip Python/shell, 10.231 baris dokumen, `config/`, `.github/`.

---

## 0. Metode dan batas verifikasi

Yang **benar-benar dijalankan** di lingkungan ini:

| Pemeriksaan | Hasil |
|---|---|
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | **98 tes lulus** |
| Parse `scripts/gui-smoke-x11.py` (AST) | valid |
| Inventaris & metrik statis (jumlah tes, panjang fungsi, referensi silang, pemindaian `unsafe`/`unwrap`/`TODO`/`allow`) | dijalankan |
| Heuristik duplikasi: 2.105 fungsi diekstrak, 264 fungsi ≥600 karakter dibandingkan per urutan token (ambang 0,72) | dijalankan |

Yang **tidak bisa dijalankan**: `rustc`/`cargo` tidak tersedia dan `static.rust-lang.org` di luar jangkauan jaringan. Jadi `cargo fmt`, `clippy -D warnings`, `cargo build`, `cargo test`, dan suite GUI X11 **tidak diverifikasi ulang di sini**. Klaim "CI hijau" di dokumen proyek tidak saya anggap bukti untuk audit ini; semua temuan Rust di bawah berbasis pembacaan kode statis.

Keterbatasan pembacaan: semua file diinventaris dan diukur; modul inti (`app.rs`, `document/state.rs`, `document/preview.rs`, `preview_*.rs`, `export/*`, `config.rs`, `commands.rs`, `main.rs`, `workspace.rs`, `search.rs`, `files.rs`, `images.rs`, `history.rs`) dibaca penuh; sisanya dibaca pada bagian kritis dan diperiksa lewat metrik.

---

## 1. Ringkasan penilaian

| Prinsip `AGENT.md` | Penilaian | Catatan singkat |
|---|---|---|
| 1. Think Before Coding | **Cukup** — 3 invariant rapuh | Asumsi "hanya satu draft terbuka" dijaga di 3 lapis dengan 4 guard yang tidak seragam; dua `Option` paralel + `expect` di jalur render |
| 2. Simplicity First | **Titik terlemah** | `update()` 408 baris; dua tipe plan ~90% identik; empat siklus draft paralel; ≈2.082 baris harness eksperimen ikut binary rilis; satu baris skrip 8.311 karakter |
| 3. Surgical Changes | **Bersih, ada sisa** | Tidak ada `unsafe`, `TODO/FIXME`, `#[allow]`, atau item `pub` mati; tetapi ada opsi konfigurasi mati, 7 salinan konstanta yang sama, dan konvensi tes yang bercampur |
| 4. Goal-Driven Execution | **Kuat di unit test, lemah di kriteria** | 1.106 tes; CI ketat; tetapi ekspor HTML tidak memakai tema (padahal roadmap kriteria #4), benchmark tanpa ambang regresi, pencarian tanpa batas atas |

Severity yang dipakai: **P0** = kontradiksi dengan klaim/dokumen atau risiko salah perilaku; **P1** = biaya pemeliharaan nyata, aman diperbaiki bertahap; **P2** = kebersihan/higiene.

Audit ini awalnya tidak mengubah kode (sesuai `AGENT.md` §3: sentuh hanya yang diminta). Perbaikan D1, B6, dan B7 menyusul dan dicatat di [§9](#9-status-perbaikan-31-okt-2026).

---

## 2. Prinsip 1 — Think Before Coding

### A1 · P1 · Invariant "satu draft saja" dijaga tidak seragam di 3 lapis

Komentar di `app.rs:294` menyatakan *"Two single-line panels never open together, so a keystroke can never be routed to the wrong local stack"*, dan `draft_history_panel()` (`app.rs:296-308`) bergantung penuh pada asumsi itu. Kenyataannya asumsi tersebut ditegakkan oleh:

1. Whitelist pesan di `Hylite::update` (`app.rs:384-470`) — 4 blok `match` terpisah.
2. Empat predikat berbeda:
   - `cell_operation_allowed` (`app/cell_edit.rs:18-24`) — **tidak** memeriksa `paragraph_draft`/`multiline_draft`/`visual_draft`.
   - `paragraph_operation_allowed` (`app/paragraph_edit.rs:58-60`) — hanya memeriksa `cell_draft`.
   - `multiline_operation_allowed` (`app/multiline_paragraph.rs:10-15`) — memeriksa cell + paragraph.
   - `visual_operation_allowed` (`app/visual_edit.rs:11-25`) — memeriksa paragraph, multiline, cell, **dan** tiga draft probe.
3. `draft_history_panel()` yang mengembalikan `None` bila lebih dari satu draft hidup (`app.rs:299-307`).

Akibatnya: bila kelak ada jalur pesan baru (atau pemanggilan handler langsung seperti di `app/tests.rs`), `cell_draft` bisa hidup bersama `paragraph_draft`, lalu `Ctrl+Z` diam-diam jatuh ke history dokumen — persis kesalahan yang diklaim mustahil.

**Saran (surgical):** satu sumber kebenaran, misalnya `enum ActiveDraft { Paragraph, Multiline, Cell, Visual }` + `fn active_draft(&self) -> Option<ActiveDraft>`; guard per modul dan `draft_history_panel` memakai itu. Menghapus 3 lapis menjadi 1.

### A2 · P1 · Dua `Option` paralel membawa satu konsep, dijaga `expect`

`document/preview.rs` meneruskan `spike: Option<&Draft>` **dan** `mode: Option<InputMode>` secara terpisah ke `render_block`, lalu memanggil:

- `mode.expect("interactive inline editor requires an experiment mode")` — `document/preview.rs:377`
- `mode.expect("active inline editor has a mode")` — `document/preview.rs:510`

Saat ini aman karena pemanggil selalu mengisi keduanya, tetapi invariant "dua Option ini setara" tidak bisa ditegakkan kompilator; pelanggaran = panic di jalur render.

**Saran:** satu nilai, mis. `enum InlineMode<'a> { Off, Stage9a(&'a Draft), Stage9b(&'a Probe) }`, atau `struct SpikeSession<'a> { draft: &'a Draft, mode: InputMode }`. `Option<SpikeSession>` tidak bisa setengah terisi.

### A3 · P2 · `start_operation` panic, bukan `Result`

`app/file_actions.rs:25-36` memakai `.expect("file operations must be checked before starting another")`. Pemanggil hari ini memang dijaga (`file_operation_available`, `pending_action`), tetapi ini invariant UI yang ditegakkan dengan panic pada jalur I/O; satu jalur baru yang lupa menjaga = crash saat menyimpan.

**Saran:** kembalikan `Option<OperationToken>`/`Result` dan perlakukan sebagai no-op + pesan status (pola yang sudah dipakai di tempat lain).

---

## 3. Prinsip 2 — Simplicity First

### B1 · P1 · `Hylite::update` 408 baris dengan 4 blok guard nyaris identik

`app.rs:310-718`. Empat blok `if self.<draft>.is_some() { if !matches!(...) { report_blocked... } }` (`app.rs:384-470`) berisi daftar pesan yang diizinkan per panel, semuanya berbeda hanya pada beberapa varian pesan. ±110 baris yang bisa menjadi satu tabel `(predikat, daftar pesan, label panel)`.

Fungsi terpanjang lain jauh lebih wajar (hanya 16 dari 2.105 fungsi >100 baris), jadi `update` adalah outlier tunggal.

### B2 · P1 · `SingleMovePlan` ≈ `MultilineMovePlan` (≈90% identik)

- Tipe: `document/state.rs:48-104` (dua struct + dua `impl` dengan accessor sama, beda hanya `move_line`).
- Logika: `prepare_single_paragraph_move` (`state.rs:623-724`, 95 baris) vs `prepare_multiline_paragraph_move` (`state.rs:842-935`, 92 baris) — rasio token **0,89**; perbedaan inti hanya validator payload dan ada/tidaknya `line`.

Estimasi: ±150-200 baris dapat disatukan dengan satu parameter payload/validator tanpa mengubah perilaku. Perlu tes regresi lebih dulu karena menyentuh jalur commit atomik.

### B3 · P1 · Empat siklus draft paralel di lapisan `app/`

Rasio terukur (token, ambang 0,72):

| Pasangan | Rasio |
|---|---|
| `app/inline_advanced_probe.rs:28 open_inline_alt_probe` ↔ `app/inline_spike.rs:89 open_inline_probe` | 0,92 |
| `app/multiline_paragraph.rs:43 move_multiline_paragraph` ↔ `app/paragraph_edit.rs:172 move_preview_paragraph` | 0,89 |
| `app/cell_edit.rs:228 history_preview_cell` ↔ `app/paragraph_edit.rs:353 history_preview_paragraph` | 0,87 |
| `app/cell_edit.rs:62 apply_preview_cell` ↔ `app/multiline_paragraph.rs:97 apply_multiline_paragraph` | 0,86 |

Empat modul (`paragraph_edit` 387, `multiline_paragraph` 268, `cell_edit` 257, `visual_edit` 163 baris) mengulang pola yang sama: `open → change → move → apply/apply-next/apply-previous → cancel → history`, dengan string notifikasi yang juga berulang ("… is stale; Cancel and reopen. Draft kept.").

Catatan penting: **jangan** langsung membuat abstraksi generik. Perbedaan semantik (satu baris vs multiline vs sel vs visual) nyata. Yang paling bernilai adalah menyatukan bagian yang benar-benar identik (history routing, notice, guard) dan membiarkan sisanya eksplisit — sesuai semangat "no abstractions for single-use code".

### B4 · P2 · Dua panel view nyaris sama

`ui/paragraph_edit.rs:81 view` ↔ `ui/multiline_paragraph.rs:197 view` — rasio **0,73**, keduanya ~100 baris, sama-sama membangun header/footer/aksi panel dengan label yang berbeda.

### B5 · P1 (butuh keputusan) · ≈2.082 baris harness eksperimen ikut binary rilis

Modul yang hanya melayani eksperimen yang sudah dinyatakan **NO-GO** di dokumen:

| File | Baris |
|---|---|
| `src/inline_edit.rs` (hanya dipakai probe) | 571 |
| `src/ui/inline_spike.rs` | 487 |
| `src/ui/inline_advanced_probe.rs` | 485 |
| `src/app/inline_spike.rs` | 415 |
| `src/app/inline_advanced_probe.rs` | 124 |
| **Total** | **≈2.082** |

Ditambah: 9 field di struct `Hylite` (`inline_*_enabled/pending/draft`), 3 flag CLI di `main.rs`, cabang di `update`/`title`/`view_preview`, 5 fungsi `render_*` di `document/preview.rs`, dan 17 tes bernama `probe`/`spike`.

`AGENT.md` §2 ("no features beyond what was asked", "nothing speculative") berbenturan dengan peran harness ini sebagai **bukti** yang dirujuk `docs/testing-stage9a.md`, `docs/testing-stage9b.md`, dan `docs/proposal-alternative-widget-probe.md` (ketiganya dihapus pada Langkah 7). Ini keputusan produk, bukan keputusan teknis — lihat §7.

### B6 · P2 · Validasi flag CLI triplikat

`main.rs:57-85`: tiga blok `if inline_* { eprintln!("usage: …") }` + tiga blok `if inline_experiment && !is_file { eprintln!(…) }`, semuanya berbeda hanya pada nama flag. ±30 baris bisa menjadi satu tabel `(flag, usage, validation message)`.

### B7 · P1 · `scripts/gui-smoke-x11.py`: satu baris 8.311 karakter

- Baris **4088** panjangnya **8.311 karakter**: rantai ternary bersarang ±30 tingkat untuk memilih string `scope` per suite.
- `exercise_suite` (`3916-4005`) adalah rantai `if/elif suite == "…"` **31 cabang**.
- Tuple `choices` 31 entri ditulis tangan di `4011`, sementara `document = …_fixture(...)` diulang per suite (`4049-4060`) dan `inline-spike` diperiksa dua kali.

Skrip 4.096 baris ini adalah gate utama proyek. Semua di atas bisa menjadi satu tabel:

```python
SUITES = {"paragraph-draft": Suite(fixture=..., run=exercise_paragraph_draft, scope="…"), ...}
```

Dengan begitu `choices`, dispatch, fixture, dan `scope` tidak mungkin lagi tidak sinkron — risiko yang sekarang nyata karena empat daftar terpisah harus diubah bersamaan.

---

## 4. Prinsip 3 — Surgical Changes

### C1 · P2 · Konstanta 16 KiB disalin 7 kali dengan 3 nama

`const … = 16 * 1024` di:
`continuation.rs:5 MAX_LINE_BYTES`, `inline_edit.rs:7 MAX_BYTES`, `preview_cell.rs:9 MAX_BYTES`, `preview_paragraph.rs:6 MAX_BYTES`, `preview_paragraph_multiline.rs:6 MAX_BYTES`, `preview_visual.rs:11 MAX_BYTES`, `syntax.rs:8 MAX_LINE_BYTES`.
Pesan pengguna juga menuliskannya literal: `"Paragraph draft exceeds 16 KiB"` dan `"escaped Markdown must fit 16 KiB"` (`app/paragraph_edit.rs:102-107`).

Batas yang sama untuk konsep berbeda boleh punya konstanta berbeda, tetapi hari ini semuanya satu angka yang sama tanpa rujukan bersama; perubahan kebijakan = 7 suntingan + beberapa string.

### C2 · P2 · Opsi konfigurasi yang tidak pernah bisa aktif

`editor.auto_save` (`config.rs:22`) selalu dipaksa `false` + peringatan (`config.rs:146-149`), dan tetap ada di `config/default.toml` sebagai dokumentasi. Ini "configurability that wasn't requested" versi `AGENT.md` §2: permukaan konfigurasi yang tidak punya perilaku. Pilihan: hapus (breaking untuk config lama, tetapi `deny_unknown_fields` sudah menolak kunci tak dikenal) atau dokumentasikan sebagai reserved secara eksplisit.

### C3 · P2 · Konvensi tes bercampur

- 44 file memakai `mod tests { … }` inline, 30 tes memakai file terpisah `*_tests.rs` — dua konvensi hidup berdampingan.
- `document/state.rs:1692-1762` memasang **19** modul tes lewat `#[path = "..."]`, sehingga `state.rs` menjadi induk semua tes preview — pola tidak lazim dan tidak konsisten dengan `preview_model_tests.rs` yang memakai `mod tests` inline.
- Nama file kembar: `src/preview_task_batch_tests.rs` dan `src/document/preview_task_batch_tests.rs` (isi berbeda, konteks berbeda) — mudah tertukar saat mencari.

Bukan bug; tetapi biaya navigasi nyata pada basis 1.106 tes.

### C4 · P2 · Fixture tes menyalin seluruh struct `Hylite`

`app/tests.rs:7-43` menulis 33 field satu per satu. Setiap field baru di `Hylite` harus diedit di dua tempat (produksi + fixture) dan kesalahannya hanya terlihat saat kompilasi tes. `Default`/konstruktor tes akan menghilangkan seluruh kategori kesalahan ini.

### Positif pada prinsip ini

- **Nol** `unsafe`, `TODO`, `FIXME`, `#[allow(...)]`, `#![...]` penekan lint di `src/`.
- **Nol** item `pub` yang tidak pernah dirujuk (pemindaian 600 simbol `pub`).
- Di luar tes hanya ada **5** `unwrap`/`expect`: `app/file_actions.rs:35` (A3), `document/preview.rs:377` & `:510` (A2), `syntax.rs:322` & `:365` (dijaga oleh `i < len`, risiko rendah).
- Tidak ada kode yang "dirapikan diam-diam": diff historis tidak bisa saya nilai, tetapi kondisi akhir tidak menunjukkan sisa refactor setengah jadi.

---

## 5. Prinsip 4 — Goal-Driven Execution

### D1 · P0 · Ekspor HTML tidak memakai tema, dan opsi parser berbeda dari Preview

- `export::to_html` memakai `Options::all()` (`export/mod.rs:8`), sedangkan `preview_model` sengaja memilih ekstensi eksplisit (`document/preview_model.rs:85-90`, dengan komentar "Export remains independent").
- `export::export_html` (`export/mod.rs:15-17`) menulis CSS **Catppuccin gelap hardcoded**; tidak ada parameter palette, dan `file_actions::begin_export` tidak mengirim tema. Bandingkan PDF: `export_cli::theme_palette(&theme)` (`export_cli.rs:21`) dan `export::export_pdf(markdown, palette, path)`.
- Roadmap 1.0 kriteria #4 berbunyi **"Ekspor PDF + HTML dengan tema"** dengan bukti **"Byte-compare HTML"** (`docs/roadmap.md:27`). Tidak ada tes yang membandingkan HTML antar-tema, dan `Options::all()` membuat hasil HTML berpotensi berbeda dari Preview (mis. smart punctuation).

Ini satu-satunya temuan yang saya nilai **P0**: kontradiksi langsung antara klaim tertulis dan perilaku kode.

**Kriteria lulus yang bisa diverifikasi:** `export_html(markdown, palette, path)` + tes "byte-compare untuk dua tema berbeda" + tes paritas opsi dengan `preview_model::options()`.

### D2 · P1 · Benchmark tidak punya ambang regresi

`performance_benchmarks.rs:12,120` hanya menegaskan total <180 detik. Angka per metrik dicetak (`open/typing/parse/render/…`) tetapi tidak dibandingkan dengan apa pun. Roadmap kriteria #2 menuntut "angka absolut … batas waktu di CI"; huruf kecilnya sudah dipenuhi, tetapi tanpa ambang per operasi, regresi 5× tetap lolos.

Catatan jujur: `docs/performance.md` sudah menyatakan ini bukan klaim performa. Jadi ini **gap kriteria**, bukan klaim palsu.

### D3 · P2 · Pencarian tidak dibatasi, berbeda dari seluruh modul lain

`search.rs:26-28` mengumpulkan **semua** `match_indices` tanpa batas, sementara proyek ini konsisten membatasi di tempat lain (512 entri workspace, 1.024 task batch, 128 baris tabel, 16 KiB draft, 2.048 byte math). Pada dokumen 5 MB dengan kueri satu huruf, `matches` bisa berisi ratusan ribu rentang, dan `refresh` dipanggil tiap ketikan.

**Saran:** tetapkan batas (mis. 10.000 match) + status "hasil dipotong", dan masukkan ke daftar batas yang sudah didokumentasikan.

### D4 · P2 · Apa yang tidak bisa diverifikasi (dan tidak boleh diklaim)

Tidak ada jalur di repositori ini yang membuktikan Wayland/Hyprland, portal, IME, HiDPI, atau packaging — dan dokumen proyek **sudah** menyatakannya berulang kali (bagus). Saya mencatatnya agar audit ini tidak dipakai untuk mengklaim lebih: suite yang ada berjalan di X11/Xvfb/tiny-skia.

### Positif pada prinsip ini

- **1.106** `#[test]`; 256 di antaranya di `app/tests.rs`, dengan nama yang mendeskripsikan perilaku.
- `config.rs:184-190` membandingkan `config/default.toml` dengan `Config::default()` — drift tertangkap tes, bukan disiplin manual.
- CI (`.github/workflows/ci.yml`) menegakkan `cargo fmt --check`, `clippy --all-targets -D warnings`, tes tanpa display, lalu 31 invokasi GUI X11 (`regression` + 30 suite khusus) — kriteria lulus yang kuat dan eksplisit.
- Planner murni (`preview_*.rs`, `text_edit.rs`, `navigation.rs`) memakai aritmetika `checked_*` dan mengembalikan `Option` alih-alih panic — pola yang tepat dan konsisten.
- Batas tabel (12 kolom/128 baris/512 sel), gambar (4 MiB/8192 px/40 MP), workspace (512/8) di kode **sama** dengan yang tertulis di dokumen.

---

## 6. Rencana perbaikan berurutan (dengan kriteria lulus)

Prinsip: satu langkah = satu jenis perubahan, tiap langkah punya verifikasi yang bisa dijalankan tanpa saya (karena toolchain Rust tidak ada di sini).

| # | Langkah | Cakupan | Verifikasi |
|---|---|---|---|
| 1 | **D1** tema HTML export + satukan opsi parser | `export/mod.rs`, `file_io.rs`, `app/file_actions.rs` | tes baru: byte-compare 2 tema; `cargo test export::` |
| 2 | **C2** hapus/tegaskan `auto_save`, **C1** satu konstanta `DRAFT_MAX_BYTES` | `config.rs`, 7 modul, `config/default.toml` | `cargo test config:: preview_*`; `cargo clippy -D warnings` |
| 3 | **A1** `ActiveDraft` tunggal, **A2** `SpikeSession`, **A3** `start_operation → Option` | `app.rs`, `app/*.rs`, `preview.rs` | tes aplikasi yang ada + tes baru "dua draft tidak bisa hidup bersamaan" |
| 4 | **B1** tabel guard `update`, **B6** tabel flag CLI, **C4** fixture `Default` | `app.rs`, `main.rs`, `app/tests.rs` | `cargo test`; tidak ada perubahan perilaku yang diharapkan |
| 5 | **B2/B3/B4** dedupe plan & siklus draft | `document/state.rs`, `app/*`, `ui/*` | tes regresi ditulis **dulu**, lalu `cargo test` + suite GUI `paragraph-*`, `table-cells`, `draft-history-*` |
| 6 | **B7** data-driven suites di skrip | `scripts/gui-smoke-x11.py` | 98 tes Python + `--suite regression` dan 2 suite lain |
| 7 | **B5** keputusan probe/spike (hapus vs `cfg(feature)`) | 5 modul + `main.rs` + dokumen | `cargo build` tanpa fitur; suite `inline-*` hanya dengan fitur |

Langkah 1-2 kecil dan berisiko rendah. Langkah 3-5 menyentuh jalur commit atomik, jadi menurut `AGENT.md` §4 perlu tes regresi **sebelum** refactor, bukan sesudahnya.

---

## 7. Keputusan yang saya butuhkan sebelum menyentuh kode

1. **Mulai dari mana?** Saran saya Langkah 1 (D1) karena itu satu-satunya kontradiksi klaim, lalu Langkah 2. Atau Anda ingin langsung ke dedupe (Langkah 5) karena biaya pemeliharaan?
2. **Probe/spike (B5):** dihapus, dijadikan `cfg(feature = "probe")` agar tidak ikut binary rilis, atau dipertahankan apa adanya karena menjadi bukti historis di `docs/`?
3. **Ekspor HTML (D1):** apakah tema harus mengikuti `Config.appearance.theme` (seperti PDF), atau ekspor HTML memang sengaja satu tema gelap dan yang perlu dikoreksi justru klaim di `docs/roadmap.md`?

Ketiga jawaban itu menentukan apakah pekerjaan berhenti di ±2 file (D1 + koreksi dokumen), ±10 file (dedupe ringan), atau ±25 file (refactor draft + pelepasan probe).

---

## 8. Lampiran — daftar temuan

| ID | Prinsip | Severity | Lokasi | Inti |
|---|---|---|---|---|
| A1 | 1 | P1 | `app/cell_edit.rs:18`, `app/paragraph_edit.rs:58`, `app/multiline_paragraph.rs:10`, `app/visual_edit.rs:11`, `app.rs:294` | Mutual exclusion draft dijaga 4 guard tak seragam |
| A2 | 1 | P1 | `document/preview.rs:377`, `:510` | Dua `Option` paralel + `expect` di jalur render |
| A3 | 1 | P2 | `app/file_actions.rs:25` | Invariant UI ditegakkan dengan panic |
| B1 | 2 | P1 | `app.rs:310-718` | `update` 408 baris, 4 blok guard duplikat |
| B2 | 2 | P1 | `document/state.rs:48-104`, `:623`, `:842` | Dua tipe plan ~90% identik |
| B3 | 2 | P1 | `app/{paragraph_edit,multiline_paragraph,cell_edit,visual_edit}.rs` | Empat siklus draft paralel (0,86-0,92) |
| B4 | 2 | P2 | `ui/paragraph_edit.rs:81`, `ui/multiline_paragraph.rs:197` | Dua panel view 0,73 |
| B5 | 2/3 | P1 | 5 modul probe, `main.rs`, `app.rs` | ≈2.082 baris eksperimen NO-GO di binary rilis |
| B6 | 2 | P2 | `main.rs:57-85` | Validasi 3 flag CLI triplikat |
| B7 | 2 | P1 | `scripts/gui-smoke-x11.py:3916`, `:4011`, `:4088` | 31 cabang, tuple 31 entri, satu baris 8.311 karakter |
| C1 | 3 | P2 | 7 modul (`MAX_BYTES` dst.) | Konstanta 16 KiB disalin 7× |
| C2 | 3 | P2 | `config.rs:22`, `:146` | `auto_save` tidak bisa aktif |
| C3 | 3 | P2 | `document/state.rs:1692-1762`, nama file kembar | Konvensi tes bercampur |
| C4 | 3 | P2 | `app/tests.rs:7-43` | Fixture menyalin 33 field |
| D1 | 4 | P0 | `export/mod.rs:7-17` vs `docs/roadmap.md:27` | HTML ekspor tanpa tema + `Options::all()` |
| D2 | 4 | P1 | `performance_benchmarks.rs:12`, `:120` | Tanpa ambang regresi per metrik |
| D3 | 4 | P2 | `search.rs:26` | Hasil pencarian tanpa batas |
| D4 | 4 | P2 | `.github/workflows/ci.yml` | Tidak ada bukti Wayland/Arch (sudah diakui dokumen) |

**Total:** 1× P0, 8× P1, 9× P2.

---

## 9. Status perbaikan (31 Okt 2026)

| ID | Status | Perubahan | Verifikasi |
|---|---|---|---|
| D1 | **selesai** | `export_html(markdown, palette, path)` dengan CSS dari warna tema; `to_html` memakai `preview_model::options()`; `theme_palette` dipusatkan di `export` sehingga dua salinan di `export_cli` dan `app/file_actions` hilang | 2 tes baru: palet terang/gelap menghasilkan byte berbeda dengan `color-scheme` yang benar; tidak ada smart punctuation |
| B6 | **selesai** | `main.rs`: tiga flag probe menjadi `enum Experiment` + `parse_experiment`; ~45 baris duplikat menjadi satu tabel | parser Rust (tree-sitter): 0 error; alur flag tetap sama |
| B7 | **selesai** | `scripts/gui-smoke-x11.py` kini punya satu tabel `SUITES`: 28 entri `Suite(scope=…, steps=(Step("exercise_…", needs=…), …), fixture=…, workspace=…)`. Tabel itu sekaligus menjadi `--suite` choices, dispatch, pemilihan fixture/argumen boot, dan `scope` di `summary.json`; rantai `if/elif suite == "…"` 28 cabang (≈85 baris) dan ternary 8.311 karakter sebelumnya sudah hilang. Nama fungsi langkah disimpan sebagai string dan di-resolve lewat `globals()`, jadi tes tetap bisa mem-patch `exercise_*` seperti sebelumnya | 93 tes Python lulus (91 + 2 tes baru); dispatch lama vs baru dibandingkan per suite dengan mem-patch setiap `exercise_*` dan merekam argumennya: identik untuk ke-28 suite, termasuk urutan langkah, `needs`, fixture, dan pesan penolakan |
| C1 | **selesai** | modul baru `src/limits.rs` memuat `DRAFT_MAX_BYTES` dan `LINE_MAX_BYTES`; tujuh modul kini merujuk satu definisi lewat nama domainnya masing-masing (`MAX_BYTES` untuk draft, `MAX_LINE_BYTES` untuk satu baris sumber), jadi perubahan kebijakan = satu suntingan. Nama publik per modul tetap, sehingga ~40 rujukan di tes tidak berubah | parser tree-sitter: 0 error; `tools/source-core-tests.rs` ikut menyertakan `limits` sehingga harness standalone tetap berdiri sendiri |
| C2 | **selesai (ditegaskan, bukan dihapus)** | `editor.auto_save` dipertahankan agar config lama tetap memuat (menghapusnya akan ditolak `deny_unknown_fields`), tetapi kini eksplisit sebagai reserved: komentar field menjelaskan alasannya, `#[serde(skip_serializing)]` membuatnya tidak pernah ditulis balik, dan `config/default.toml` mengganti barisnya dengan catatan reserved | tes config menambah satu asersi: setelah `auto_save = true` dinormalkan, hasil serialisasi tidak memuat `auto_save`; tes bundled-config tetap membandingkan `config/default.toml` dengan `Config::default()` |
| A3 | **selesai** | `start_operation` mengembalikan `Option<OperationToken>` dan melakukan pemeriksaan ketersediaan sendiri, jadi `expect` hilang. Penjaga duplikat di pemanggil (`OpenAt`, `begin_save`, `begin_export`, `begin_export_pdf`) dihapus; kelima tempat pemanggilan memakai `let Some(token) = … else { return Task::none() };`. Jalur baru yang lupa menjaga kini menjadi no-op + status "Wait for the current file operation to finish", bukan panic di jalur I/O | tes baru `starting_a_second_operation_is_a_no_op_with_a_status_message`; parser tree-sitter: 0 error |
| A2 | **selesai** | `document/preview.rs`: `struct SpikeSession<'a> { draft: &'a Draft, mode: InputMode }` menggantikan pasangan `spike` + `mode`. `render_preview_internal`, `render_blocks`, dan `render_block` menerima satu `Option<SpikeSession<'a>>`; kedua `expect` dihapus karena nilai setengah terisi tidak lagi bisa dibentuk. Dispatch dan label tombol "Finish … (Esc)" kini berasal dari nilai yang sama. Signature publik `render_inline_spike_preview`/`render_inline_probe_preview` tidak berubah | parser tree-sitter: 0 error; kedua pemanggil publik membangun sesi dari `Draft`/`Probe` sehingga perilaku tetap |
| A1 | **selesai** | `enum ActiveDraft { Paragraph, Multiline, Cell, Visual }` + `Hylite::active_draft()` sebagai satu sumber kebenaran; `draft_history_panel()` memetakan nilai itu. Semua pemeriksaan "ada draft lain?" kini memakai nilai yang sama: `source_edit_allowed`, `preview_history_allowed`, `preview_task_batch_action`, `tab_switch_allowed` (sebelumnya melewatkan `visual_draft`), tiga guard probe, dan `ui::editing::view`. Catatan penting: predikat `*_operation_allowed` **tidak** memuat `active_draft()` karena dipakai ulang oleh operasi *di dalam* draft (`apply_*`, `move_*`, `format_*`); aturan "satu draft" dipindah ke guard `open_*` yang kini seragam berbentuk `active_draft().is_some() \|\| !<predikat>`. `restore_input_focus` dan label judul tetap memilih per jenis karena memang per jenis | tes baru `one_active_draft_owns_the_keyboard_and_refuses_other_panels`: draft sel menolak panel paragraf, yang dulu tidak memeriksa draft sel sama sekali; parser tree-sitter: 0 error |
| B1 | **selesai** | Empat blok guard di `Hylite::update` (±90 baris) menjadi satu gerbang `if let Some(active) = self.active_draft()`. `ActiveDraft::accepts(&message)` memuat keempat daftar pesan sebagai satu `match`, dan `ActiveDraft::label()` memberi label status yang sama seperti sebelumnya. Daftar izin per panel diperiksa identik dengan versi lama oleh skrip pembanding: visual 9 pesan, multiline 11, paragraf 12, sel 12 — termasuk `Cancel*` serta `DraftHistory` yang hanya dibagi tiga panel single-line, bukan panel multiline | 0 perubahan perilaku yang diharapkan; seluruh tes Rust lulus |
| C4 | **selesai** | `impl Default for Hylite` memuat keadaan "buffer kosong + config default + tanpa draft"; `app()` di `app/tests.rs` menyusut dari 33 field menjadi `Hylite { document: Document::new("Original\n"), ..Hylite::default() }`, jadi field baru tidak perlu ditambahkan di dua tempat | seluruh tes Rust lulus |
| B2 | **selesai** | `SingleMovePlan`/`MultilineMovePlan` menjadi satu `ParagraphMovePlan`; `prepare_single_paragraph_move`/`prepare_multiline_paragraph_move` (±190 baris) menjadi satu `prepare_paragraph_move`, begitu pula kedua `commit_*`. Bedanya kini eksplisit di `enum ParagraphRoot { Single, Multiline }`: validator payload, `plan`, `owns`/`is_root`/`payload_matches`. `move_line()` mengembalikan `Option<usize>` karena hanya editor satu baris yang melacak nomor baris; pemanggil di `app/paragraph_edit.rs` menyesuaikan. Nama fungsi publik (`prepare_*`, `commit_*`) dan signature-nya tidak berubah | dua tes regresi baru menuliskan perilaku yang tidak boleh kabur: plan satu baris menolak root multiline (dan sebaliknya), dan `move_line()` hanya terisi untuk plan satu baris |
| B5 | **selesai — dihapus** | Atas keputusan pemilik repo, seluruh harness eksperimen dihapus: `src/inline_edit.rs`, `src/ui/inline_spike.rs`, `src/ui/inline_advanced_probe.rs`, `src/app/inline_spike.rs`, `src/app/inline_advanced_probe.rs`, `SpikeSession` + dua jalur render di `document/preview.rs`, 9 field + 3 konstruktor + 3 blok dispatch di `app.rs`, 5 varian `Message` + 2 tipe event di `commands.rs`, `enum Experiment`/`parse_experiment` di `main.rs`, 8 tes aplikasi, 7 tes Python, dan 3 langkah CI (`--suite inline-spike`, `inline-probe`, `inline-alt-probe`) | `grep` tanpa sisa di `src/`, `scripts/`, `.github/`, `tools/`; 91 tes Python lulus; tree-sitter 0 error; fmt/clippy/tes Rust lulus di CI |
| B3 | **selesai sebagian** | Dua dari empat pasangan hilang bersama B5 (modul probe). Yang tersisa disatukan hanya sebatas yang identik: `apply_multiline_paragraph_previous` + `apply_multiline_paragraph_next` (2×64 baris) menjadi satu `apply_multiline_paragraph_adjacent(direction)` dengan tabel `AdjacentLabels` berisi enam laporan yang memang per arah; `record_of` yang disalin verbatim di `app/paragraph_edit.rs` dan `app/cell_edit.rs` kini satu `fn record_of` di `app.rs`. Siklus `open → change → move → apply → cancel` per jenis draft **tidak** dijadikan abstraksi generik: validasi, pemulihan fokus, dan bentuk penolakan memang berbeda (cell memakai `Task::none()`, paragraph/multiline memakai `restore_input_focus`) | build, clippy, dan seluruh tes Rust lulus; 14 tes Apply & Next/Previous multiline yang sudah ada mengunci setiap string dan urutan langkah |
| B4 | **selesai** | `ui/layout::draft_panel()` menjadi satu-satunya definisi kartu panel 820 px yang sebelumnya disalin di `ui/paragraph_edit.rs`, `ui/multiline_paragraph.rs`, dan `ui/cell_edit.rs` (±12 baris × 3). Geometri, padding bertingkat, dan `max_width` tetap 820 (`NARROW_BREAKPOINT`) | tes UI ketiga modul dan suite X11 `paragraph-*` lulus; tidak ada perubahan tata letak |
| C3 | **selesai (tanpa migrasi massal)** | `src/preview_task_batch_tests.rs` → `src/preview_task_batch_plan_tests.rs`, jadi tidak lagi kembar dengan `src/document/preview_task_batch_tests.rs`; mount `#[path]` di `preview_task.rs` ikut disesuaikan. Konvensi ganda (44 `mod tests` inline vs 30 file `*_tests.rs`) **tidak** diseragamkan: memindahkan kode tes tidak mengubah perilaku dan hanya menambah risiko. Aturannya kini tertulis di `docs/architecture.md` §"Konvensi tes": mulai inline, pindah ke file terpisah bila melewati ~200 baris, dan nama file harus menyebut levelnya | build + seluruh tes Rust lulus; tidak ada perilaku yang berubah |
| D2 | **selesai** | `performance_benchmarks.rs` menambahkan `CEILINGS_MS` (5× median terburuk di `docs/performance.md`) yang membandingkan **median** tiap metrik, plus dua assertion struktural yang tahan mesin lambat: `incremental_preview` < `fallback_preview` dan `render_tree` < `parse`. Status 180 detik tetap ada | step CI *Measure Stage 10b raw editor timings* **benar-benar dijalankan** dan lulus (run [41264823377](https://github.com/Indrawan007/Hylite/actions/runs/41264823377), step 12 setelah saya pindahkan sementara ke depan suite X11 yang sedang flaky, lalu dikembalikan) |
| D3 | **selesai** | `search.rs`: `MAX_MATCHES = 10_000` + satu match probe untuk membedakan "tepat 10.000" dari "lebih banyak"; panel menampilkan `…/10000+`; **Replace All ditolak** saat hasil terpotong (mengganti sebagian prefix akan meninggalkan sisa dokumen tanpa kabar), sedangkan Replace tunggal dan navigasi tetap bekerja pada hasil terkumpul. Batas ini didokumentasikan di `docs/architecture.md` bersama batas lain | tes baru di `search.rs` (cap, tepat-cap, reset, penyempitan query) dan tes aplikasi `replace_all_refuses_a_truncated_result_instead_of_replacing_a_prefix`; seluruh tes Rust lulus di CI |
| D4 | **tidak ada tindakan** | Sudah berupa pernyataan batas; tidak ada klaim Wayland/portal/IME/HiDPI/packaging di repo ini | — |

Verifikasi perubahan ini:

| Pemeriksaan | Hasil |
|---|---|
| `cargo fmt --all -- --check`, `clippy --all-targets -D warnings`, build, seluruh tes Rust — run [41203900828](https://github.com/Indrawan007/Hylite/actions/runs/41203900828) (`d236d02`, D1+B6+B7) dan run [41245392225](https://github.com/Indrawan007/Hylite/actions/runs/41245392225) (`c12f09e`, C1+C2) | **lulus** |
| Langkah 3 (A1+A2+A3) — run [41251781097](https://github.com/Indrawan007/Hylite/actions/runs/41251781097) (`e708641`) | **fmt, clippy, build, seluruh tes Rust lulus**; satu-satunya merah adalah flake X11 yang sudah tercatat di [§9a](#9a-gate-gui-x11-yang-merah-sebelum-sesi-ini) (`Native table preview and wheel input leave Split Source byte-exact`) |
| Langkah 4 (B1+C4) — run [41252415257](https://github.com/Indrawan007/Hylite/actions/runs/41252415257) (`2163719`, B1) dan [41252802892](https://github.com/Indrawan007/Hylite/actions/runs/41252802892) (`1547b29`, C4) | **fmt, clippy, build, seluruh tes Rust lulus**; satu-satunya merah tetap flake X11 §9a (`Warm Split: table cell edit …` lalu `Native table preview and wheel input …`) |
| 98 tes Python (`unittest discover -s scripts`) | lulus |
| Parser Rust tree-sitter pada file yang diubah tiap langkah | 0 error sintaks |
| `B7`: nilai `scope` 31 suite dibandingkan dengan ekspresi lama | identik, termasuk urutan |
| Langkah 7 (B5) — run [41260341053](https://github.com/Indrawan007/Hylite/actions/runs/41260341053) (`37dd1ab`) dan [41260653002](https://github.com/Indrawan007/Hylite/actions/runs/41260653002) (`3a2e34c`) | **format, clippy `-D warnings`, build, seluruh tes Rust, dan 91 tes Python lulus**; satu-satunya merah tetap flake X11 §9a (`Native table preview and wheel input leave Split Source byte-exact` — check yang sama sudah merah pada `2b2845a`, sebelum B5) |
| 91 tes Python setelah B5 (`unittest discover -s scripts`) | lulus (dari 98; 7 tes probe ikut dihapus) |
| `grep -rn "spike\|probe" src/ scripts/ .github/ tools/` | tidak ada sisa |
| Langkah 7 (C3+D2+D3) — run [41264500903](https://github.com/Indrawan007/Hylite/actions/runs/41264500903) (`b437b4f`) dan [41264823377](https://github.com/Indrawan007/Hylite/actions/runs/41264823377) (`769b8a6`) | format, strict core, clippy `-D warnings`, build, dan seluruh tes Rust lulus; benchmark Stage 10b lulus dengan ambang baru; satu-satunya merah tetap flake X11 §9a. Satu kesalahan nyata di tengah jalan: tes cap men-shadow helper `search()` sehingga gagal kompilasi — dibetulkan di `b437b4f` |
| Langkah 6 (B7 remainder) — run [41263231847](https://github.com/Indrawan007/Hylite/actions/runs/41263231847) (`37f29d2`) | format, strict core, clippy `-D warnings`, build, dan seluruh tes Rust lulus; suite GUI X11 utama berjalan lewat tabel baru dan lulus sampai flake §9a berikutnya (`Warm Split: table cell edit is byte-exact after Go to Line`, bukan check yang sama dengan run sebelumnya), sehingga langkah suite 14–41 kembali ter-skip — dispatch per-suite karena itu dikunci oleh tes Python, bukan oleh run ini. `unittest discover -s scripts`: **93 tes lulus**; dua tes baru mengunci tabel (`SUITES` = satu sumber untuk choices/dispatch/fixture/scope, setiap helper `exercise_*` terjangkau, tidak ada entri menggantung) dan argumen boot tiap suite (folder workspace, dokumen fixture, atau tanpa argumen) beserta `scope` di `summary.json` |
| Langkah 5b (B3+B4) — run [41262129327](https://github.com/Indrawan007/Hylite/actions/runs/41262129327) (`dd5b7ac`) dan [41262295227](https://github.com/Indrawan007/Hylite/actions/runs/41262295227) (`115dc53`) | **format, strict core, clippy `-D warnings`, build, dan seluruh tes Rust lulus**; satu-satunya merah tetap flake X11 §9a. Dua koreksi nyata di tengah jalan: tiga hunk rustfmt (baris kosong sisa, dua rantai `history.step(...)`) dan satu `dead_code` nyata — `Record::at_end` tidak terpakai ketika `tools/source-core-tests.rs` mengompilasi `multiline_draft_history` sendiri, jadi helper-nya pindah ke `app::record_of` |

Catatan verifikasi Langkah 3: dua run di antaranya (`41250413374`, `41250584210`) merah di *Check formatting* dan satu (`41251074070`) di *Lint all targets*. Keduanya cacat nyata pada perubahan A2, bukan flake: rustfmt ingin `render_blocks(blocks, colors, snapshot, base_dir, session, &inline_edit)` dalam satu baris setelah argumen `spike`+`mode` menyusut menjadi satu `session`, dan `on_block` memindahkan `block.source` (`Range<usize>` bukan `Copy`) sehingga gagal kompilasi. Log job tidak dapat diunduh dari sandbox, jadi diff-nya dibaca lewat langkah CI sementara yang mencetak output sebagai anotasi; langkah itu sudah dihapus lagi.

### 9b. Penghapusan harness eksperimen (Langkah 7)

Keputusan produk: **hapus** (bukan `cfg(feature)`), karena verdict Stage 9a/9b/probe sudah NO-GO atau terbatas dan tidak ada jalur dari harness ke fitur. Yang hilang dari binary: 5 modul (`inline_edit`, `ui/inline_spike`, `ui/inline_advanced_probe`, `app/inline_spike`, `app/inline_advanced_probe`), 9 field status, tiga konstruktor `with_inline_*_path`, `enum Experiment` + `parse_experiment` beserta flag `--inline-edit-spike`/`--inline-feasibility-probe`/`--inline-alt-widget-probe`, lima varian `Message` probe, dan tiga suite GUI. Yang tetap: seluruh jalur Preview/visual-edit, inline *preview text* (`OpenInlinePreviewText`), serta suite GUI lain (28 suite).

Kerja mekanis yang tetap diperlukan setelah penghapusan: `cargo fmt --check` menolak tiga hal yang tersisa — baris kosong ekstra di akhir `app/tests.rs` (bekas pemotongan), `format!` judul dan rantai `text(...)` di Preview yang jadi terlalu panjang setelah cabang probe hilang, dan `match &block.kind` di `render_block` yang kini muat satu baris sehingga seluruh cabangnya harus di-indent ulang satu level. Ketiganya dibereskan di `37dd1ab`.

Konsekuensi yang disengaja:
- Refactor A2 (`SpikeSession`) ikut terhapus. Perbaikannya tetap tercatat sebagai temuan yang sah — invariant "dua `Option` paralel" memang nyata — tetapi objeknya kini tidak ada.
- Dokumen `docs/testing-stage9a.md`, `docs/testing-stage9b.md`, dan `docs/proposal-alternative-widget-probe.md` dihapus. Semua tautan ke ketiganya kini menunjuk ke bagian ini; riwayat angka run tetap ada di git, di `docs/verification-native.md`, `docs/testing-gui-x11.md`, `docs/progress-report.md`, dan `docs/roadmap.md` yang diberi banner "dihapus/historis".
- Tidak ada perubahan perilaku pada fitur yang tersisa.

### 9a. Gate GUI X11 yang merah sebelum sesi ini

Langkah *Smoke test actual GUI on isolated X11* sudah gagal pada commit `98a419d` (hanya dokumen) dan pada commit-commit sebelumnya di branch ini, jadi kegagalan itu bukan akibat perbaikan audit. Penyelidikan dengan anotasi check-run (log job tidak dapat diunduh dari sandbox) menemukan **race pra-perubahan**, bukan regresi deterministik:

- Titik gagal **berpindah** antar-run (`Find first`, `Unicode and trailing tabs: paste`, `Go to Line: Escape …`, `Native table preview …`), termasuk pada commit yang `src/`-nya identik byte-per-byte.
- Trace aplikasi sementara menunjukkan tombolnya sampai dan `Message::Edit(SelectAll)` diterapkan, tetapi salinan tidak pernah terjadi (`marker-owner-exit=None`) atau berisi buffer yang kehilangan karakter sentinel — yaitu karakter yang diketik **satu frame setelah panel Find/Go To Line ditutup** mendarat di input panel yang belum di-unmount, lalu dibuang oleh guard `GoToQueryChanged`/`FindQueryChanged`.
- Suite juga berjalan sampai ~178 s terhadap anggaran 180 s, sehingga frame yang lambat memperbesar jendela race.

Perbaikan yang dipilih (hanya harness, kode aplikasi tidak disentuh — `git diff 3021b03 -- src/` kosong):

| Perubahan harness | Alasan |
|---|---|
| `expect_copy` mengulang maksimal 3 gesture dengan marker di-stage ulang | hand-off clipboard X11 asinkron; assertion byte-exact dan aturan "satu Copy setelah sukses" tetap |
| Klik refocus pindah dari x=600 (batas pane Split) ke x=500, diklik ulang tiap percobaan + jeda satu frame | 600 adalah piksel batas: pembulatan menentukan apakah klik mencapai editor |
| `settle()` satu frame setelah tombol yang menutup panel/draft, di 4 titik yang langsung diikuti pengetikan sentinel | menghilangkan kelas race "karakter tertelan input panel yang belum di-unmount" |
| Anggaran suite 180 s → 300 s, `timeout-minutes` step 5 → 10 | suite memang sudah menyentuh 178 s pada runner ini |
| Pesan gagal mencetak status marker-owner dan window title | kegagalan berikutnya bisa didiagnosis dari anotasi CI saja |

Hasil: gate naik dari gagal di cek ke-21 menjadi lolos 81-88 cek (seluruh cek Go to Line dan Find yang semula merah sekarang lulus). Sisa satu cek flaky di suite tabel (`Native table preview and wheel input leave Split Source byte-exact`): aplikasi tidak menyalin sama sekali dan title normal tanpa draft/konfirmasi, konsisten dengan `Ctrl+Shift+M` (Preview→Split) yang kadang tidak berefek pada iterasi itu; cek yang sama lulus di run lain. Ini **temuan terbuka** untuk langkah berikutnya, bukan bagian dari D1/B6/B7/C1/C2.

### 9c. Flake X11: akar masalah urutan pesan (1 Nov 2026)

Sesi lanjutan mengejar flake §9a sampai akar penyebabnya, bukan hanya titik gagalnya. Semua bukti di bawah berasal dari anotasi check-run (log job dan artefak screenshot tidak dapat diunduh dari sandbox).

**Akar masalah (terbukti).** `iced` 0.13 menguras pesan widget (teks yang diketik) **sebelum** pesan keyboard dari subscription jendela (`AboutToWait` → `ui.update` → `update(messages)`; subscription masuk lewat `Action::Output` pada siklus berikutnya). Selama aplikasi sibuk merender tabel native, chord dan teks sesudahnya jatuh di batch yang sama, sehingga `Ctrl+Shift+M` diterapkan **setelah** teks. Terukur di run `41268401052`: dokumen berisi `LEAK` dan mode baru berubah sesudahnya — teksnya menggantikan seluruh seleksi yang ditinggalkan `Ctrl+A` dari cek salinan tepat sebelumnya. Karena Preview read-only hanya dijaga oleh "tidak ada widget editor", teks itu tetap masuk ke dokumen.

**Perbaikan kode aplikasi (`9560c1d`).**
- `editor::key_binding` menerbitkan chord kontrol (`Ctrl`+karakter) sebagai `Binding::Custom` dari key press widget itu sendiri, sehingga urutannya terhadap teks di batch yang sama terjaga.
- `commands::subscription_shortcut` melewati chord yang sudah di-capture editor (status `Captured`), supaya satu gestur tetap satu perintah; tombol bernama (Escape, F3, F5, F11) tetap lewat subscription agar binding native-nya (Escape = unfocus) tidak berubah.
- Tes unit baru di `src/editor.rs` dan `src/commands.rs`; dua tes lama disesuaikan karena `Ctrl+F/H/G` dan `Ctrl+B` kini juga berupa `Custom` (tetap bukan penyisipan karakter).

**Pengerasan harness (`07d23c9`…`38403c6`, `707167a`).**

| Perubahan | Alasan terukur |
|---|---|
| `expect_copy` mencatat per percobaan isi clipboard + window title di pesan gagal | kegagalan berikutnya menjelaskan dirinya sendiri lewat anotasi |
| `expect_copy` menunggu owner clipboard keluar (bounded) sebelum gestur berikutnya | hand-off X11 masih berjalan saat tombol berikutnya dikirim |
| `mode_is_editable` / `cycle_to(editable=…)` | `Ctrl+Shift+M` bisa hilang; mode diverifikasi lewat dokumen (Preview read-only tidak bisa menyalin), bukan diasumsikan |
| `undo_until_clean()` menggantikan pasangan `Ctrl+Z` + `title_is_dirty(False)` | satu undo bisa hilang; judul yang memutuskan, bukan jumlah tombol |
| `settle()` sebelum klik tombol action bar Preview (heading, setext, reveal, cell, task) | klik yang dikirim satu frame setelah pergantian mode di-hit-test pada frame lama dan mendarat di teks blok, membuka draft |
| Go To Line di suite cache: `settle()` lalu `Ctrl+A` sebelum mengetik alamat | input panel baru ter-mount satu frame kemudian; karakter pertama tertelan sehingga alamat ditambahkan, bukan menggantikan seed |

**Status gate.** Step 12 kini melewati seluruh suite `exercise_preview`, `exercise_tables`, `exercise_preview_cache`, dan `exercise_preview_reveal`. Titik gagal terakhir yang tersisa ada di suite tabel/riwayat dan bersifat intermiten (satu gestur hilang); rinciannya di §9d.

**Temuan terbuka.**
1. `Ctrl+Shift+M` yang dikirim saat Source tidak fokus (Preview) masih kadang hilang; perbaikannya saat ini hanya pemulihan di harness (`cycle_to`), bukan alasan yang terbukti di aplikasi.
2. Koordinat produk pada suite reveal/cell/task/heading/setext perlu dipetakan ulang terhadap build sekarang; sampai itu selesai, suite-suite tersebut tidak dapat dipakai sebagai bukti regresi.
3. Kelas "satu keystroke tertelan" (§9a) belum diperbaiki di aplikasi; harness hanya mengulang gestur.

Seluruh langkah §6 (1–7) sudah dijalankan dan tercatat di §9. Satu-satunya temuan yang sengaja ditutup tanpa perubahan kode adalah C3 (konvensi tes diseragamkan lewat dokumentasi, bukan migrasi massal) dan D4 (pernyataan batas, bukan cacat).

### 9d. Lanjutan 1 Nov 2026: reveal membangun editor yang ia fokuskan

**Temuan produk (terukur).** Klik bilah "Show block source" pada Preview memang berpindah ke Split, tetapi editor yang baru dibangun **tidak menerima fokus**: operasi fokus dijalankan pada frame yang masih menampilkan Preview, yang tidak punya widget editor. Terlihat sebagai `TimeoutError` pada cek `Preview source action opens Split and focuses exact block-start caret` — tidak ada widget yang bisa menyalin sama sekali, bukan salinan yang salah.

**Perbaikan (`6ef3f4f`, `7e1489b`).** `app::navigation::reveal_preview` mengembalikan `Task::batch([restore_input_focus(), Task::done(Message::RestoreInputFocus)])`; `Message::RestoreInputFocus` adalah pesan fokus idempoten yang diproses paling awal di `update` (di luar guard draft, karena ia tidak menyentuh dokumen). Percobaan yang sama pada `SwitchMode`/`CycleMode` sudah dicoba lalu **dibatalkan**: jalur itu sudah diverifikasi harness lewat `cycle_to`, dan fokus yang datang terlambat justru memperlebar jendela saat draft dibuka.

**Geometri bilah (terukur, bukan asumsi).** Dengan probe yang hasilnya tidak bisa tertukar antar-percobaan, klik pada kolom x=800 di y=151 me-reveal blok dengan awal provenance **byte 26** (paragraf `body **bold**`, bukan heading byte 0), sedangkan y=139/147/157/165 tidak me-reveal apa pun pada kolom itu. Jadi bilah tiap blok berada tepat di atas isi blok itu, dan `(600,151)` yang dipakai sebelumnya bukan bilah blok mana pun.

**Pengerasan harness tambahan.**

| Perubahan | Alasan terukur |
|---|---|
| `copy_probe()` mem-poll clipboard (1,5 dtk) dan melempar bila tidak ada jawaban | aplikasi menyajikan seleksi secara asinkron; satu pembacaan langsung masih melihat marker sendiri, dan `None` dari xclip sempat dibaca sebagai "dokumen editable" |
| `reveal_from_preview(points, source, offset)` | klik yang meleset bisa membuka draft (widget editable), jadi "ada yang editable" bukan bukti; sentinel harus mendarat di `offset` di depan dokumen utuh. Setiap offset yang teramati dilaporkan, draft dibatalkan, sentinel di-undo |
| `toolbar_history(point, label, expected)` | klik toolbar yang dikirim sebelum frame toolbar digambar sama dengan meleset; tombol ditekan ulang hanya selama dokumen belum menjawab |
| `settle()` setelah `cycle_to(editable=True)` pada suite tabel | widget difokuskan oleh frame sesudah siklus mode, bukan oleh siklus itu sendiri |

**Peta bilah aksi Preview (terukur, fixture heading).** Probe yang mengklasifikasi lewat dokumen (bukan lewat mode, karena perubahan heading ** tetap di Preview) memberi, pada y=150: x=800 → `Show block source`, x=858 → `H-`, x=950 dan x=980 → tombol draft (`Edit visually`), x=830/890/920/1010 → tidak menjawab (tekanan hilang, kelas §9a). Konsekuensinya klik lama `932`/`984` mendarat di kontrol draft — itulah sebabnya draft teks visual terbuka saat yang diharapkan perubahan level heading. Suite heading dan setext kini menekan `(906,150)` untuk H+ dan `(858,150)` untuk H− lewat `press_until_document()`.

**Anggaran waktu.** Suite kini memverifikasi setiap gestur lewat dokumen, sehingga melewati batas lama 300 dtk (batas itu sendiri pernah dinaikkan dari 180 dtk dengan alasan yang sama) dan sebuah run sehat dilaporkan sebagai `GUI smoke exceeded its … global budget`. Batas naik ke 600 dtk (langkah CI 15 mnt) dan setiap baris `PASS:` kini membawa cap waktu `[ 190.0s]` supaya biaya per cek terbaca dari anotasi. Pada run `41284531065` tidak ada `PASS:` baru selama 410 dtk setelah cek cache di 190 dtk, jadi masih ada satu titik yang benar-benar menggantung — belum teridentifikasi karena log job dan artefak tidak dapat diunduh dari sandbox.

**Temuan terbuka (setelah sesi ini).**
1. Kelas "satu gestur hilang" (§9a) tetap belum diperbaiki di aplikasi: `Ctrl+Shift+M` dan `Ctrl+A`/`Ctrl+C` masih kadang tidak sampai. Harness mengulang gestur dan memverifikasi lewat dokumen.
2. Gate belum pernah hijau penuh dalam satu run. Suite yang sudah dipastikan lulus pada minimal satu run: tabel, cache, reveal, cell, task (sebagian), heading (H+/H− kini di kolom terukur).
3. Ada satu penggantungan >400 dtk setelah suite cache yang belum terjelaskan; tanpa log job/artefak, penyebabnya hanya bisa ditebak.
4. Artefak screenshot tidak bisa diunduh dari sandbox ini, sehingga pemetaan koordinat produk hanya bisa lewat anotasi (satu run per pertanyaan).

### 9e. Lanjutan 1 Nov 2026: Paste yang tidak pernah sampai, dan bilah yang berpindah baris

**Dua kegagalan berbeda pada suite yang sama.** Run `41293606397` dan `41295689067` mati di `exercise_preview_reveal`, tetapi bukan karena sebab yang sama.

1. `TimeoutError: Window dirty title = True` sesudah `paste_from_clean()`: jendela masih berjudul `Untitled — Hylite`, jadi tidak ada `Ctrl+V` yang pernah diproses. Pemeriksaan yang ada hanya membuktikan judul **bersih**, dan judul bersih juga benar ketika tidak satu pun tombol sampai ke aplikasi — Paste yang hilang karena jendela tidak fokus dan Paste yang ditolak editor terlihat identik.
2. `AssertionError: Preview block action revealed [], not byte 26`: tidak satu pun klik menghasilkan jawaban editable, jadi yang hilang adalah **bilahnya**, bukan offsetnya.

**Perbaikan 1 (harness).** `paste_from_clean()` menegaskan fokus X (`xdotool windowfocus --sync`) lalu mengulang gestur sekali; judul kotor tetap menjadi syarat sebelum `expect_copy` mengambil alih clipboard. Suite reveal juga tidak lagi mewarisi mode dari suite sebelumnya: ia mencapai Preview lewat `cycle_to(editable=False)` (probe read-only), bukan satu `Ctrl+Shift+M` yang mengasumsikan serah-terima Source. Terbukti pada run berikutnya: judul menjadi `Untitled • — Hylite` dan kegagalan berpindah ke cek sesudahnya.

**Perbaikan 2 (harness).** Baris bilah blok ditentukan tinggi toolbar di atasnya. `(800,151)` terukur pada build `e5d5d82`; setelah tutup tab memakai ikon SVG (tombol setinggi 28 px) baris itu bukan bilah blok mana pun, dan `y=139/147/157/165/175` juga tidak. Karena itu `reveal_from_preview()` menerima **kolom** calon baris (`y=104..314`, langkah 6, x=800) dan dokumen tetap satu-satunya saksi: klik meleset di Preview read-only gratis (jawaban `PROBE_MARKER`), klik yang me-reveal blok lain di-undo lalu dilaporkan offsetnya, dan assertion tetap **byte 26**. Koordinat bukan lagi bagian dari spesifikasi; awal blok yang dipindai itulah yang diuji.

**Struktur UI (`f7693bc`).** Sidebar dipisah menjadi seksi `DOKUMEN` dan `WORKSPACE` dengan garis rambut, tombol `Folder` pindah dari header `FILES` ke header `WORKSPACE` yang dibukanya, dan empty state diringkas menjadi dua baris. Toolbar diberi garis pemisah (`chrome::divider_style`) antar kelompok ikon pada setiap density yang menaruhnya dalam satu baris; baris kedua sengaja tidak disentuh karena sudah berupa panel berbingkai **dan** harness menekan tombolnya di koordinat tetap (`(29,62)`, `(71,62)`, `(185,62)`, `(305,62)`).

**Catatan geometri.** Struktur sidebar tidak menggeser satu pun koordinat harness: satu-satunya klik di separuh kiri adalah panel draft (`(225,364)`, `(425,364)`), dan saat draft terbuka `view()` mengganti seluruh isi jendela tanpa sidebar. Suite workspace sendiri menggerakkan pohon lewat keyboard (`Alt+J/K/Enter`), bukan klik baris.

**Status gate.** Run `41295689067` (sha `4c0a65e`): langkah 6–11 hijau (1090 + 338 tes, clippy, fmt) dan langkah 12 masih merah pada sebab (2). Perbaikan (2) diuji oleh run `7f0b172`.

### 9f. Peta baris bilah Preview yang terukur (1 Nov 2026)

Penyapuan kolom x=800 pada run `41297006756` (sha `ccc83e0`) memberi peta yang tidak lagi ambigu, karena setiap jawaban kini mencatat byte di sekitar caret dan judul jendela:

| y (x=800) | jawaban | konteks | judul | arti |
|---|---|---|---|---|
| 146, 152, 158, 164 | 0 | `X# café` | `Untitled • — Hylite` | bilah **source** blok heading (awal blok = 0, benar) |
| 230, 242 | 9 | `ody boldX` | `Untitled • [visual text draft] — Hylite` | bilah **draft visual** paragraf (teks draft `body bold`, caret di ujung) |
| 284–302 | 28 | `bold**\n\nX\| Name` | `Untitled • — Hylite` | bilah **source** blok tabel (awal blok = 28, benar) |

Jadi pada kolom x=800 yang terjangkau hanya bilah source **heading** dan **tabel**; paragraf `body **bold**` menampilkan bilah draft visual saja, tidak bilah source-nya.

**Koreksi assertion (harness).** Anggapan lama bahwa paragraf itu mulai di byte 26 salah: heading `# café🙂` hanya 11 byte UTF-8, jadi paragraf mulai di byte 13 dan byte 26 berada di dalam baris kosong sebelum tabel — offset yang tidak mungkin dihasilkan implementasi yang benar. Suite reveal kini menekan bilah source **tabel** dan menuntut caret tepat di byte 28 (`source.index(b"| Name")`), dengan pindaian `y=104..334` di x=800; jawaban 0 dari heading dan 9 dari draft tetap dilaporkan sebagai bukti, bukan diterima.

**Biaya.** Satu jawaban salah = Escape + undo sentinel + `cycle_to(editable=False)` (±5–10 dtk); suite berjalan ±105 dtk pada run `41297006756`, masih jauh di bawah anggaran 780 dtk.

