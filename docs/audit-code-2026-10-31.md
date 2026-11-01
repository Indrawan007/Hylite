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
| B7 | **selesai sebagian** | `gui-smoke-x11.py`: `SCOPE_BY_SUITE` + `DEFAULT_SCOPE` menggantikan ternary 8.311 karakter; `choices` dan `scope` kini berasal dari satu tabel | 98 tes Python lulus; nilai `scope` 31 suite dibandingkan dengan versi lama: identik termasuk urutan; `--help` tetap 31 pilihan |
| C1 | **selesai** | modul baru `src/limits.rs` memuat `DRAFT_MAX_BYTES` dan `LINE_MAX_BYTES`; tujuh modul kini merujuk satu definisi lewat nama domainnya masing-masing (`MAX_BYTES` untuk draft, `MAX_LINE_BYTES` untuk satu baris sumber), jadi perubahan kebijakan = satu suntingan. Nama publik per modul tetap, sehingga ~40 rujukan di tes tidak berubah | parser tree-sitter: 0 error; `tools/source-core-tests.rs` ikut menyertakan `limits` sehingga harness standalone tetap berdiri sendiri |
| C2 | **selesai (ditegaskan, bukan dihapus)** | `editor.auto_save` dipertahankan agar config lama tetap memuat (menghapusnya akan ditolak `deny_unknown_fields`), tetapi kini eksplisit sebagai reserved: komentar field menjelaskan alasannya, `#[serde(skip_serializing)]` membuatnya tidak pernah ditulis balik, dan `config/default.toml` mengganti barisnya dengan catatan reserved | tes config menambah satu asersi: setelah `auto_save = true` dinormalkan, hasil serialisasi tidak memuat `auto_save`; tes bundled-config tetap membandingkan `config/default.toml` dengan `Config::default()` |
| A3 | **selesai** | `start_operation` mengembalikan `Option<OperationToken>` dan melakukan pemeriksaan ketersediaan sendiri, jadi `expect` hilang. Penjaga duplikat di pemanggil (`OpenAt`, `begin_save`, `begin_export`, `begin_export_pdf`) dihapus; kelima tempat pemanggilan memakai `let Some(token) = … else { return Task::none() };`. Jalur baru yang lupa menjaga kini menjadi no-op + status "Wait for the current file operation to finish", bukan panic di jalur I/O | tes baru `starting_a_second_operation_is_a_no_op_with_a_status_message`; parser tree-sitter: 0 error |
| A2 | **selesai** | `document/preview.rs`: `struct SpikeSession<'a> { draft: &'a Draft, mode: InputMode }` menggantikan pasangan `spike` + `mode`. `render_preview_internal`, `render_blocks`, dan `render_block` menerima satu `Option<SpikeSession<'a>>`; kedua `expect` dihapus karena nilai setengah terisi tidak lagi bisa dibentuk. Dispatch dan label tombol "Finish … (Esc)" kini berasal dari nilai yang sama. Signature publik `render_inline_spike_preview`/`render_inline_probe_preview` tidak berubah | parser tree-sitter: 0 error; kedua pemanggil publik membangun sesi dari `Draft`/`Probe` sehingga perilaku tetap |
| A1 | **selesai** | `enum ActiveDraft { Paragraph, Multiline, Cell, Visual }` + `Hylite::active_draft()` sebagai satu sumber kebenaran; `draft_history_panel()` memetakan nilai itu. Semua pemeriksaan "ada draft lain?" kini memakai nilai yang sama: `source_edit_allowed`, `preview_history_allowed`, `preview_task_batch_action`, `tab_switch_allowed` (sebelumnya melewatkan `visual_draft`), tiga guard probe, dan `ui::editing::view`. Catatan penting: predikat `*_operation_allowed` **tidak** memuat `active_draft()` karena dipakai ulang oleh operasi *di dalam* draft (`apply_*`, `move_*`, `format_*`); aturan "satu draft" dipindah ke guard `open_*` yang kini seragam berbentuk `active_draft().is_some() \|\| !<predikat>`. `restore_input_focus` dan label judul tetap memilih per jenis karena memang per jenis | tes baru `one_active_draft_owns_the_keyboard_and_refuses_other_panels`: draft sel menolak panel paragraf, yang dulu tidak memeriksa draft sel sama sekali; parser tree-sitter: 0 error |
| B1 | **selesai** | Empat blok guard di `Hylite::update` (±90 baris) menjadi satu gerbang `if let Some(active) = self.active_draft()`. `ActiveDraft::accepts(&message)` memuat keempat daftar pesan sebagai satu `match`, dan `ActiveDraft::label()` memberi label status yang sama seperti sebelumnya. Daftar izin per panel diperiksa identik dengan versi lama oleh skrip pembanding: visual 9 pesan, multiline 11, paragraf 12, sel 12 — termasuk `Cancel*` serta `DraftHistory` yang hanya dibagi tiga panel single-line, bukan panel multiline | 0 perubahan perilaku yang diharapkan; seluruh tes Rust lulus |
| C4 | **selesai** | `impl Default for Hylite` memuat keadaan "buffer kosong + config default + tanpa draft"; `app()` di `app/tests.rs` menyusut dari 33 field menjadi `Hylite { document: Document::new("Original\n"), ..Hylite::default() }`, jadi field baru tidak perlu ditambahkan di dua tempat | seluruh tes Rust lulus |
| B2 | **selesai** | `SingleMovePlan`/`MultilineMovePlan` menjadi satu `ParagraphMovePlan`; `prepare_single_paragraph_move`/`prepare_multiline_paragraph_move` (±190 baris) menjadi satu `prepare_paragraph_move`, begitu pula kedua `commit_*`. Bedanya kini eksplisit di `enum ParagraphRoot { Single, Multiline }`: validator payload, `plan`, `owns`/`is_root`/`payload_matches`. `move_line()` mengembalikan `Option<usize>` karena hanya editor satu baris yang melacak nomor baris; pemanggil di `app/paragraph_edit.rs` menyesuaikan. Nama fungsi publik (`prepare_*`, `commit_*`) dan signature-nya tidak berubah | dua tes regresi baru menuliskan perilaku yang tidak boleh kabur: plan satu baris menolak root multiline (dan sebaliknya), dan `move_line()` hanya terisi untuk plan satu baris |
| B5 | **selesai — dihapus** | Atas keputusan pemilik repo, seluruh harness eksperimen dihapus: `src/inline_edit.rs`, `src/ui/inline_spike.rs`, `src/ui/inline_advanced_probe.rs`, `src/app/inline_spike.rs`, `src/app/inline_advanced_probe.rs`, `SpikeSession` + dua jalur render di `document/preview.rs`, 9 field + 3 konstruktor + 3 blok dispatch di `app.rs`, 5 varian `Message` + 2 tipe event di `commands.rs`, `enum Experiment`/`parse_experiment` di `main.rs`, 8 tes aplikasi, 7 tes Python, dan 3 langkah CI (`--suite inline-spike`, `inline-probe`, `inline-alt-probe`) | `grep` tanpa sisa di `src/`, `scripts/`, `.github/`, `tools/`; 91 tes Python lulus; tree-sitter 0 error; fmt/clippy/tes Rust lulus di CI |
| B3-B4, C3, D2-D4 | terbuka | — | — |

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

Yang sengaja belum disentuh: B3-B4 (siklus draft di `app/` dan dua panel view — perlu tes regresi lebih dulu), C3 (higiene), D2-D4. B5 (probe/spike) sudah diputuskan pemilik repo: **hapus seluruhnya** — lihat [§9b](#9b-penghapusan-harness-eksperimen-langkah-7).
