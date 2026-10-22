# Stage 5a — fondasi preview Markdown native read-only

Source tetap otoritatif; ini **bukan WYSIWYG** dan bukan pernyataan Tahap 4 telah
selesai. Tidak ada perubahan serializer, format save, undo/history, atau worker
file/ekspor. Tidak menambah dependensi maupun mengubah `Cargo.lock`.

## Yang diimplementasikan

| Struktur | Perilaku preview |
| --- | --- |
| Heading ATX/setext | Level 1–6, ukuran/weight berbeda, inline style di dalamnya |
| Paragraph | Rich text, soft break menjadi spasi, hard break menjadi newline |
| Strong/emphasis/strike | Font bold/italic gabungan dan dekorasi strike native; state luar dipulihkan setelah nested style ditutup |
| Inline code | Font monospace dan background; markup di dalamnya literal |
| Link/autolink/reference link | Label dan gaya inline, warna tema + underline; metadata URL di model, **tanpa aksi membuka URL** |
| Bullet/ordered list | Struktur child blocks, indent dan nomor awal ordered dipertahankan |
| Task list | Penanda `[x]`/`[ ]` read-only pada item yang tepat, bukan checkbox editable |
| Quote | Panel berindentasi; paragraph/list/code anak tidak dipipihkan |
| Fenced/indented code | Info string + panel monospace, payload parser tidak di-trim; terminal spasi/tab/LF tidak sengaja dibuang |
| Rule | Garis native |
| Tema | Warna text/link/panel diturunkan dari tema Iced aktif, bukan warna fixed-dark |

Fenced code mengikuti payload CommonMark parser; indented code menghapus indent
Markdown sesuai parser. Ini bukan klaim glyph/tab-stop/line-wrap identik dengan
Source atau pixel layout telah diverifikasi. Code panjang masih memakai layout
text native; belum ada syntax highlighting bahasa, editor code, atau scroll
horizontal khusus.

### Fallback eksplisit dan inert

- Tabel, HTML block, dan definisi footnote: panel berlabel dengan raw source.
- Inline HTML, math, dan referensi footnote: source literal/monospace, bukan
  HTML execution atau KaTeX.
- Gambar: placeholder `image not loaded` beserta source Markdown. Tidak membaca
  path lokal, mem-fetch URL, atau mengirim request jaringan.
- Nesting block/inline lebih dalam dari 24: source fallback berlabel; tail setelah
  subtree tetap diproses. Ini batas kedalaman model, bukan batas resource total.

Tidak ada browser/WebView, link launch callback, atau content mutation dalam
renderer. Source URL yang berbahaya pun hanya data/gaya, bukan aksi. Ekspor HTML
menggunakan jalur renderer tersendiri dan tidak diubah oleh Stage 5a.

## Model dan provenance

`document/preview_model.rs` memisahkan parsing dari widget. Node dan inline run
memiliki source range byte UTF-8. Preview memakai `Document::buffer_text()`,
bukan `text()` yang mengikuti konvensi serializer menambahkan final LF.
Tooltip block menampilkan rentang tersebut sebagai provenance.

**Range bukan source↔caret mapping**: entity, escape, line break dan marker
Markdown tidak punya korespondensi glyph/offset satu-ke-satu. Belum ada editable
AST, hit testing, selection mapping, synchronized scroll, cache preview,
incremental parse, atau virtualisasi. Parse/model/widget dibangun ulang per view.

## Gate dan cakupan pengujian

- **23 tes model**: heading, inline nesting/restorasi, code/link/reference,
  soft/hard break, list tight/loose/nested/task, quote, exact code payload,
  table/HTML/image/math/footnote fallback, malformed input, Unicode provenance,
  determinisme, dan limit kedalaman block/inline tanpa menelan tail.
- **3 tes adapter renderer**: kombinasi font; warna/dekorasi native serta tidak
  memasang action link pada tema Light/Dark; pembangunan owned widgets untuk
  nested/unsupported content. Ini bukan visual/pixel oracle.
- **1 tes controller/view**: Preview/Split/Source tidak mengubah raw/serialized
  text, snapshot, render key, undo/redo, cursor/selection, atau dirty. Skenario
  mencakup dirty dengan undo tersedia dan clean dengan redo tersedia.
- Runner GUI X11 menambahkan **4 pemeriksaan clipboard** atas fixture sebenarnya:
  Source sebelum mode change; input read-only tidak bocor saat Preview; exact
  source/terminal whitespace sesudah cycle; satu Undo kembali ke welcome clean.
  Source diklik kembali setelah perpindahan mode, bukan klaim auto-focus.
  Screenshot preview, fallback setelah wheel, dan Split disimpan tanpa assertion
  pixel. Seluruh gate navigasi/lifecycle lama tetap aktif.

### Status eksekusi

Lokal: 23 tes model lulus melalui rustc dengan source parser/dependensi dari tag
GitHub resmi pada versi yang sama dengan lockfile (tanpa fitur HTML/getopts yang
tidak diperlukan model). 150 tes core Source, 14 Python helper, format dan
`git diff --check` juga lulus. Harness model sementara tidak mengganti gate Cargo
penuh; renderer Iced dan controller menunggu hasil build/Clippy/Cargo/GUI strict
pada CI. Hasil CI akan dicatat setelah benar-benar diamati.

### Gate standar

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --locked --all-targets --all-features
cargo test --locked --all-targets --all-features
./scripts/test-gui-x11.sh --artifacts /path/to/fresh/evidence
```

Runner wajib memakai Xvfb dan XDG privat sebagaimana
[protokol GUI X11](testing-gui-x11.md). Tidak memakai konfigurasi desktop atau
file pengguna dan tidak membuka portal dialog.

## Checklist manual berikutnya — belum dinyatakan lulus

- [ ] Tinjau hierarki heading, bold/italic/strike, wrap, nested list/quote dan
  code/tab pada Light/Dark dengan font nyata; bandingkan screenshot secara visual.
- [ ] Verifikasi tema lain, zoom/HiDPI, resize, dokumen panjang, interaksi wheel
  serta caret/viewport Source pada Split di Arch + Hyprland/Wayland.
- [ ] Uji IME/RTL dan grapheme shaping; provenance byte bukan layout oracle.
- [ ] Tentukan fondasi source↔selection mapping sebelum prototipe WYSIWYG.

Portal file, save-before-close, runtime compositor/GPU, packaging Arch dan
kesiapan produksi tetap di luar bukti Stage 5a ini.
