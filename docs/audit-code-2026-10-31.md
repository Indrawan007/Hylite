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

Tidak ada kode yang saya ubah pada audit ini (sesuai `AGENT.md` §3: sentuh hanya yang diminta).

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

`AGENT.md` §2 ("no features beyond what was asked", "nothing speculative") berbenturan dengan peran harness ini sebagai **bukti** yang dirujuk `docs/testing-stage9a.md`, `docs/testing-stage9b.md`, dan `docs/proposal-alternative-widget-probe.md`. Ini keputusan produk, bukan keputusan teknis — lihat §7.

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
