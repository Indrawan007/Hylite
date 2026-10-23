# Arsitektur: Source 4a–4j, preview 5a–5e, prototipe edit/history 6a–7b

Stage 6h: build debug, Clippy dan **611 tes Cargo headless lulus** pada CI Ubuntu 24.04 / Rust stable. **33 helper Python serta 116 regresi + 9 batch + 9 alignment + 9 row + 14 cell draft checks GUI X11/Xvfb + tiny-skia** juga lulus; lihat [cell draft](testing-stage6h.md), [body row tabel](testing-stage6g.md), [alignment tabel](testing-stage6f.md), [batch task](testing-stage6e.md), [heading Setext](testing-stage6d.md), [heading ATX](testing-stage6c.md), [history task](testing-stage6b.md), [prototipe task](testing-stage6a.md), [navigasi sel](testing-stage5e.md), [navigasi blok](testing-stage5d.md), [cache/fokus mode](testing-stage5c.md), [tabel](testing-stage5b.md), [fondasi renderer](testing-stage5a.md), [riwayat native](verification-native.md) dan [cakupan GUI](testing-gui-x11.md). Ini bukan kelulusan desktop Wayland/Arch/Hyprland, visual/pixel oracle atau benchmark speedup.

Perluasan [lifecycle X11](testing-lifecycle-x11.md) membuktikan New/Cancel/Discard dan native close dirty/clean. Setelah view konfirmasi penuh dihapus, `restore_input_focus` memilih query Go to Line/Find yang masih terbuka atau operasi scoped Source; New juga mengembalikan fokus input. Title menyertakan aksi konfirmasi agar status terlihat di window manager. Akselerator Alt+S/Alt+D dikirim ke controller (hanya efektif saat pending/ready), disaring dari fallback Source, dan tidak memakai Ctrl+Alt/AltGr sebagai app command. Tidak ada perubahan serializer/save worker karena penambahan lifecycle ini.

## Batas tanggung jawab

- **`app.rs`**: state UI dan pemilihan mode, routing message, subscriptions, dan layout utama. Tidak memelihara salinan terpisah dari teks/path/dirty/revisi.
- **`app/file_actions.rs`**: koordinasi dialog/worker, pengamanan aksi destruktif, dan penerapan hasil operasi. Berinteraksi dengan model dokumen dan tracker lewat API mereka.
- **`app/editing.rs`**: guard bersama untuk mutasi Source (prompt/Open/Preview), orkestrasi undo/redo, formatting, pencarian, dan replacement. Save/ekspor biasa tidak memblokir edit. Pengecualian task/heading/table/paragraph 6a–7b mempunyai guard provenance sendiri; history Preview hanya boleh melewati edge tervalidasi di puncak, tidak membuka native typing/formatting Preview.
- **`navigation.rs` / `app/navigation.rs` / `ui/navigation.rs`**: parsing koordinat dan state murni / koordinasi guard-caret-fokus / panel Go to Line.
- **`preview_navigation.rs`**: target snapshot+range+origin (blok/sel) dan validasi UTF-8 murni; Document menambahkan membership check pada model cached, controller mengatur guard, mode dan fokus.
- **`preview_heading.rs`**: planner satu byte hash untuk root ATX level 1–6; Document memeriksa warm root membership, renderer memakai metadata marker cached, bukan body display.
- **`preview_setext.rs`**: planner penggantian run underline root Setext H1↔H2, preserving panjang/indent/raw body/suffix, tanpa konversi ATX. Field metadata dan message terpisah, guard/Atomic commit/history sama dengan heading ATX.
- **`preview_task.rs`**: target item/marker/checked/snapshot dan planner satu byte; planner batch mengganti hanya middle bytes yang perlu berubah pada rentang source verbatim. Document memeriksa exact cached membership (daftar lengkap untuk batch), dengan cap 1024 sebelum jalur Atomic Source.
- **`preview_table_edit.rs`**: shared Alignment dan planner token delimiter root; full snapshot/column/count/marker/expected alignment. Document menuntut warm root Table membership, bukan pola lexical. Commit Atomic `PreviewTable` hanya mengubah colon, tanpa serialisasi sel.
- **`preview_table_row.rs`**: whole LF-terminated pipe-leading physical body line planner; Duplicate/Delete dengan full snapshot/table/index/count/columns/line. Warm root model mengautentikasi physical line terhadap projected row, cap lama berlaku, header/delimiter tidak ditargetkan. Memakai Atomic `PreviewTable` lama.
- **`preview_cell.rs` / `app/cell_edit.rs` / `ui/cell_edit.rs`**: pure payload ranges/validator/planner; controller modal draft lifecycle; raw single-line input view. Draft bukan document buffer. Apply mengautentikasi warm root cell/shape/snapshot dan melakukan satu Atomic `PreviewTable`; Cancel/no-op/invalid tidak menambah history.
- **`line_edit.rs`**: transaksi raw Source untuk Duplicate/Delete/Move Lines; rentang baris bersama dengan formatter dan indent.
- **`commands.rs`**: enum message dan aksi yang ditunda, serta pemetaan shortcut aplikasi. Binding editing native berada di `editor.rs` agar shortcut tidak menelan input biasa/AltGr.
- **`document/state.rs`**: memiliki satu buffer `text_editor::Content`, path aktif, ID dokumen, revisi operasi monotonik, checkpoint konten, checkpoint terakhir disimpan, dan bounded history. Edit efektif melalui `apply`/`change`; penggantian dokumen melalui `replace`; hasil save melalui `mark_saved`.
- **`document/history.rs`**: dua stack snapshot teks/caret/checkpoint, grouping insert/deletion tervalidasi, batas gabungan langkah dan byte teks. Entry menyimpan tag provenance preview edit edge yang ikut bergerak/prune bersama record; eligibility Preview hanya melihat puncak stack. Tidak tahu widget atau path.
- **`search.rs`**: match literal non-overlap berupa rentang UTF-8, di-cache berdasarkan snapshot penuh dan query; navigasi serta transaksi replacement tanpa widget.
- **`text_edit.rs`**: transformasi source murni untuk inline markup/indent, validasi UTF-8, serta rentang baris logis bersama.
- **`block_edit.rs`**: command leksikal heading/list/quote, link satu baris, dan fenced code block. Tidak bergantung Iced; bukan AST-aware.
- **`syntax.rs`**: lexer read-only tanpa dependensi GUI; range UTF-8 berurutan/non-overlap, state fence sebelum setiap baris, batas panjang/lookahead inline, dan invalidasi settings/render key.
- **`highlighting.rs`**: adapter `iced::advanced::text::Highlighter` ke lexer melalui `TextEditor::highlight_with`. Menetapkan warna saja, tidak font atau konten. Tidak membutuhkan fitur/dependensi `iced_highlighter` tambahan.
- **`document/snapshot.rs`**: tipe data identitas revisi yang tidak bergantung pada Iced. Dapat dipakai tracker tanpa meminjam widget atau teks.
- **`operations.rs`**: memastikan satu operasi file aktif dan setiap hasil cocok dengan token serta jenis operasinya. Token lama tidak bisa mengonsumsi operasi baru.
- **`file_io.rs`**: dialog rfd dan blocking I/O melalui worker Tokio. Mengembalikan `Result<Option<T>, String>`: `None` adalah cancel, `Err` adalah kegagalan dengan konteks.
- **`files.rs`**: penulisan atomik dan pemeriksaan alias file. Tidak mengetahui state UI.
- **`config.rs`**: memuat konfigurasi dengan default per field, validasi, dan peringatan. Tidak menimpa file yang invalid. Pengujian menggunakan path eksplisit, bukan perubahan environment global.
- **`source_focus.rs`**: operasi fokus Source idempoten melalui scope container bernama (API publik Iced), bukan `focus_next` yang dapat melepas fokus editor yang sudah fokus. Hanya menyentuh subtree Source; panel query berada di luar scope. Unit test memeriksa pengulangan, scope nested/anonim, dan scope yang tidak ada.
- **`editor.rs`**: enum mode/tema yang bisa diserialisasi, konversi font saat startup, dan binding editing native yang hanya aktif saat editor fokus. Source dan Split menggunakan builder view source yang sama.
- **`ui/confirmation.rs` dan `ui/notifications.rs`**: hanya presentasi serta penerbitan message; tidak menulis file atau mengubah dokumen secara langsung.

## Preview native read-only (Stage 5a)

`document/preview_model.rs` mengubah stream offset `pulldown-cmark 0.12.2` menjadi
model owned: block, child blocks, inline runs dan gaya gabungan. Parser tidak
bergantung pada Iced. Opsi extension eksplisit (strike, task, table, footnote,
math); tidak memakai `Options::all`. Nesting block dan stack gaya inline dibatasi
24; bagian lebih dalam tampil sebagai source literal, bukan hilang. Batas ini
bukan budget CPU/memori total atau jaminan untuk file besar.

`document/preview.rs` meminjam model untuk membangun widget Iced, bukan HTML/WebView.
Font strong/emphasis/code digabung, strike/underline memakai dekorasi span native,
heading memakai ukuran per level. List/quote mempertahankan child blocks;
code payload tidak di-trim. Warna berasal dari tema aplikasi. Tidak ada callback
link, image loader, HTML execution atau worker I/O. Span inline tetap inert; tombol provenance 5d hanya navigasi. Checkbox task 6a dan level root ATX 6c menjadi pengecualian explicit edit yang divalidasi controller/Document, bukan mutasi AST.
Fallback unsupported tetap terlihat sebagai source, bukan dipipihkan/dihapus.

`app::view_preview` mengambil proyeksi dari Document yang pada cache miss memakai
**buffer_text raw**, bukan serializer yang dapat menambah LF. Source dan Split tetap menggunakan satu native buffer otoritatif.
Rentang UTF-8 di model/tooltip merupakan provenance; entity, escape dan soft break
mengubah display text. Ini belum AST editable, WYSIWYG, hit-testing caret,
selection mapping atau sinkronisasi scroll. Mulai 5c, parsing hanya pada cache
miss dan renderer meminjam payload model. Widget tetap dibangun per view; belum
ada incremental parsing, layout cache atau virtualisasi.
Lihat [cakupan, batas, dan bukti pengujian](testing-stage5a.md).

## Tabel native (Stage 5b)

`document/preview_table.rs` dipanggil untuk `Tag::Table` dari parser block.
`Kind::Table` memuat alignment, row header/body, dan sel dengan inline runs;
setiap row/sel memiliki source range. Parser GFM menentukan padding sel kosong
(rentang dapat kosong di akhir row) dan mengabaikan body cell ekstra hanya dalam
preview. Source dan rentang row tetap mempertahankan seluruh byte.

Grid `preview_table_view.rs` meminjam model untuk membangun row teks dan layer divider
native. Row teks menentukan tinggi maksimal terlebih dahulu; overlay divider
mengikuti tinggi itu tanpa menyebarkan Fill-height ke outer vertical scroll.
Kolom fixed 160 logical px termasuk padding, alignment native per kolom,
header bold dan background tema. Horizontal scroll lokal menampung tabel lebar;
inline link/image/HTML tetap inert melalui renderer span yang sama.

Per tabel: maksimal 12 kolom, 128 row termasuk header, dan 512 sel termasuk
empty/padded/header. Salah satu limit terlampaui berarti seluruh source tabel
menjadi fallback literal; tail setelah tabel/quote tidak tertelan. Ini batas
eager widget count, bukan memori/CPU total atau budget global seluruh dokumen.
Belum auto-size kolom, editing tabel, virtualisasi atau source↔caret mapping.
Lihat [cakupan dan batas Stage 5b](testing-stage5b.md).

## Cache proyeksi dan mode fokus (Stage 5c)

Document memiliki satu `OnceCell<Vec<Block>>` lazy, terpisah dari native Content.
`preview_blocks()` melakukan raw snapshot + parse di closure cache miss saja.
`advance_revision()` menggabungkan kenaikan revision dan `take()` model lama;
semua edit efektif/transaction dan undo/redo melewati jalur ini. Replace membuat
Document baru (cache kosong), termasuk teks identik. Masa berlaku model adalah
revisi konten saat ini, bukan checkpoint yang bisa dikunjungi ulang oleh undo.

Caret/selection/no-op/save/tema/layout/mode mempertahankan model. Khusus
`set_caret`, native buffer dapat direkonstruksi dan render key highlighting
berubah, tetapi byte Source tidak berubah sehingga model tetap valid. Parser
menghasilkan model owned; view paragraph/list/table/code/fallback meminjam teks
model lewat lifetime Rust. Tidak ada clone AST per view, counter produksi,
unsafe, async parse, atau RefCell guard dalam view. Widget/span vectors/label
masih dibangun ulang dan engine Iced dapat menyalin teks untuk layout sendiri.

Cache single-entry bukan byte-bounded: satu model bisa bertahan di Source hingga
edit/replace berikutnya. History budget bukan budget cache/RAM. Edit saat Split
tetap full parse di thread UI pada view berikutnya; tidak ada angka speedup atau
latensi/RSS yang sudah diukur. [Bukti dan batas cache](testing-stage5c.md).

Mode switch/cycle menjalankan `restore_input_focus()` pada tree baru: Preview
no-op, query Go to Line/Find bila masih terbuka, selain itu scoped Source.
Probe keyboard-only menemukan kehilangan fokus Split → Source dan perbaikan
menggunakan operasi idempoten yang sama, bukan klik fallback atau `focus_next`.
Getter/cache serta task fokus tidak menaikkan snapshot atau dirty/history.

## Navigasi provenance ke Source (Stage 5d)

Renderer menerima snapshot konten saat membangun view. Hanya leaf non-empty
menampilkan **Show block source**, menerbitkan `RevealPreview(Target)` yang
berisi snapshot penuh dan range. Quote/list/item memakai target anak; tabel
menggunakan satu target whole-block, bukan sel. Range paragraph sintetis tight
list bisa mulai pada run, bukan prefix/marker container.

`Target::offset` menolak snapshot stale serta range kosong/invalid/non-UTF-8.
`Document::preview_source_offset` juga menuntut cache warm dan exact membership
pada leaf model saat ini; range sembarang yang in-bounds tidak cukup. Tidak ada
parse untuk message cold, offset rebasing atau perkiraan berdasarkan glyph.
Edit/undo/redo/replace menggugurkan target lama; caret/save metadata tidak.

Controller memakai guard navigasi (konfirmasi/Open), menolak mode Source dan
mengizinkan Save/Export tanpa mengonsumsi operasinya. Setelah `set_caret` sukses,
panel query ditutup, current Find dibersihkan, mode menjadi Split dan task scoped
Source dijalankan. Byte source, checkpoint/revisi/dirty/undo/redo tidak berubah;
selection dikolapskan, typing group ditutup dan render key bisa berganti.
Lookup model serta rekonstruksi buffer masih O(n). Tidak ada AST editing,
glyph hit testing, source↔selection mapping, reverse navigation atau scroll sync.
GUI membuktikan klik first-heading dan keyboard caret/fokus awal blok; viewport
blok jauh dan layout glyph/nested controls masih checklist manual.
Lihat [kontrak, tes dan batas Stage 5d](testing-stage5d.md).

## Navigasi sel tabel (Stage 5e)

`Target::origin` membedakan Block dari TableCell. Target sel membawa parent
range tabel, row dan column zero-based (header = row 0); caption menjelaskan
interaksi klik kanan. `mouse_area` membungkus box konten/padding sel yang punya
rentang non-empty, tanpa baris tombol per sel. Padded/empty range tidak punya
action, sementara whitespace source non-empty tetap bisa menjadi anchor.
Grid/Stack/divider/alignment dan batas tabel tetap; lookup tidak mengedit model.

Bounds core menuntut parent UTF-8 range valid yang mengandung range sel. Document
memeriksa exact parent-table membership, indeks dan range sel di model warm;
role Block tidak menerima range sel dan role Cell tidak menerima fallback
literal. Stale snapshot ditolak sebelum raw snapshot dialokasikan. Jalur guard,
caret/fokus/penutupan panel 5d dipakai ulang, dengan status asal table cell.
Snapshot, dirty, history/redo dan cache tidak berubah karena navigasi.

GUI menambahkan klik kiri + sentinel LEAK yang tidak boleh menavigasi, lalu klik
kanan Count dan sentinel X sebelum marker raw **3** setelah prefix café🙂. Empat
check clipboard membuktikan Source/fokus/dua Undo; bukan hit-testing glyph atau
scroll/selection sync. Native Document menguji header/body/nested/forged/stale
secara terpisah; GUI belum membuktikan klik nested/whitespace/scroll jauh atau
layout desktop target. [Kontrak dan batas Stage 5e](testing-stage5e.md).

## Prototipe task checkbox (Stage 6a)

`Kind::Item` kini memuat optional TaskMarker: checked state dan optional canonical
range. Core menormalisasi prefix event 0–3 spasi sebelum `[ ]`/`[x]`/`[X]`;
unsupported marker seperti tab tetap terlihat sebagai checkbox disabled. Target
UI menyertakan full snapshot, exact item range, marker range dan expected state.
Planner memeriksa byte/UTF-8/state; Document juga menuntut cache warm serta exact
membership pada task item saat ini. Stale/cold/forged/code targets tidak di-rebase.

Pada 6a, hanya `TogglePreviewTask` boleh mengedit dari Preview/Split dengan guard prompt/
Open dan penolakan mode Source. Save/Export biasa tetap diizinkan. Planner
mengganti satu byte melalui jalur commit Source, jadi Atomic history, checkpoint/
revisi/dirty, redo branching, invalidasi cache dan caret reconstruction memakai
jalur Source yang sudah ada. Caret setelah karakter toggle, selection collapse;
mode tetap, search di-refresh, query/Source focus dipulihkan sesuai konteks Split.
Tidak ada edit AST atau serialisasi Markdown; seluruh byte lain dipertahankan.

Native typing/formatting Preview tetap ditolak. Source/Split memulihkan
source/caret lama (termasuk X kapital) lewat Undo/Redo umum. Stage 6a belum
membuka history Preview; kebijakan successor 6b di bawah menambah pengecualian
sempit. GUI membuktikan checkbox Preview/Split, satu transaksi per klik,
Undo/Redo dan fokus Source tanpa refocus; checkbox nested/disabled/query-focus
runtime/viewport/Wayland belum dinyatakan lulus.
[Kontrak, hasil dan checklist Stage 6a](testing-stage6a.md).

## History Preview task-scoped (Stage 6b)

`History::Entry` menyimpan Record + bool `preview_task` untuk edge transaksi,
bukan menebak provenance dari diff satu byte. Toggle tervalidasi saja memakai
`EditKind::PreviewTask` yang Atomic tanpa coalescing. Undo/redo membawa tag yang
sama ke stack lawan; trim, branch replacement dan reset membuangnya bersama
record. Batas gabungan 200 entries/8 MiB teks tidak berubah; bool/padding bukan
bagian budget teks. Eligibility hanya O(1) top entry, tidak mencari task lebih
tua melewati Source edit atau menyimpan stack/set task paralel.

`PreviewHistory(redo)` terpisah dari `Undo/Redo` Source. Subscription menerima
exact Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y hanya pada event Ignored; Source/query yang
Captured tidak dikirim lagi. Controller mensyaratkan Preview, tanpa prompt/Open
atau query panel, serta task edge pada arah terkait. Toolbar memakai guard yang
sama. Save/Export biasa diizinkan. Restore lewat jalur history lama mengembalikan
caret/checkpoint, menaikkan revision, menginvalidasi cache dan mempertahankan
saved-checkpoint semantics. Tidak switch mode atau meminta fokus Source.
Native typing/formatting serta Undo/Redo history Source tetap diblokir di
Preview. [Desain, tes dan batas Stage 6b](testing-stage6b.md).

## Kontrol level heading root ATX (Stage 6c)

`Block::heading_marker` hanya diisi untuk Kind::Heading pada depth root dengan
marker ATX valid. Setext, list/quote, code/HTML dan fallback tetap tanpa target.
`preview_heading` memvalidasi physical line, indent 0–3 spasi, 1–6 hash, separator,
range UTF-8 dan expected level. Planner hanya menyisipkan/menghapus hash terakhir
pembuka; tidak menyusun ulang body inline, closing hash atau whitespace.

Target full snapshot/blok/marker/level harus merupakan exact root member model
warm. `AdjustPreviewHeading` ditolak di Source/prompt/Open; Save/Export boleh.
Commit privat memakai `EditKind::PreviewHeading`, Atomic/non-coalescing, dan
rekonstruksi Source lama. Revision/caret/cache/dirty/search terbarui; Preview tidak
switch/focus Source, Split memakai logical Source/query focus yang sama dengan
task. H−/H+ ditempatkan pada bar provenance lama, tinggi bar tetap 24 px; endpoint
H1/H6 dan unsupported heading tidak menerbitkan command arah yang tidak valid.

**Successor kebijakan 6b:** PreviewHistory menerima checkbox dan heading-level
edge yang tervalidasi, bukan task-only lagi. Tag Entry sekarang `preview_edit`;
masih O(1) top-only, ikut record saat transfer/trim/branch/reset. Tidak mencari
melewati Source hash edit yang tampak identik atau membuka history umum.
Semua native Preview typing/formatting guard dan Ignored-only shortcut tetap.
[Kontrak, bukti dan batas Stage 6c](testing-stage6c.md).

## Kontrol level heading Setext root (Stage 6d)

`Block::setext_marker` terpisah dari `heading_marker` ATX. Root Heading yang
bukan ATX mencoba scanner underline Setext; nested/code/HTML/rule tetap tanpa
target. Scanner memvalidasi range UTF-8, full final underline line, 0–3 spasi
indent, run satu atau lebih `=` untuk H1 / `-` untuk H2, suffix space/tab dan
body nonblank. Scanner lexical tidak membuktikan konteks root: Document tetap
menuntut exact warm root membership dan full Snapshot saat command diterima.

`AdjustPreviewSetext` memakai handler/target sendiri; adapter ATX tetap menolak
Setext seperti 6c. Renderer mencoba action Setext sebagai fallback pada kontrol
H−/H+ yang sama. H1 hanya boleh demote ke H2, H2 hanya promote ke H1, tanpa
konversi ke ATX/paragraph. TextChange mengganti run underline saja dengan panjang
sama, preserving body multiline/inline raw/indent/suffix. Commit masih
`PreviewHeading` Atomic; history top-only tidak berubah. Mode/guard/token/save/
cache/search sama dengan ATX; caret ditempatkan di akhir underline run agar
Split dapat menerima keyboard Source setelah fokus scoped dipulihkan.

GUI byte-exact memeriksa perubahan/Undo/Redo dan sentinel Source, tanpa refocus
click, bukan bukti pixel/layout/Wayland. Tidak mengubah grammar/options parser,
serializer, lockfile atau worker file. [Kontrak dan bukti Stage 6d](testing-stage6d.md).

## Batch task kanonis yang didukung model (Stage 6e)

`BatchTarget` membawa snapshot, daftar lengkap Target kanonis source-order
(termasuk anggota no-op), dan desired checked state. `canonical_preview_tasks`
hanya membaca model warm, mengabaikan unsupported marker dan mengembalikan None
bila lebih dari 1024, bukan subset/prefix. Apply menuntut equality seluruh daftar,
lalu planner memvalidasi setiap token/snapshot/expected state, order dan marker
non-overlap. Item range nested boleh overlap; marker range tidak boleh.

Planner menyalin rentang middle-byte pertama hingga terakhir yang berubah,
memodifikasi hanya byte ASCII `x`/space yang perlu, lalu mengembalikan satu
TextChange (panjang sama). `[X]` yang sudah checked tidak dinormalisasi. Satu
commit `PreviewTask` Atomic, bukan loop single-task; Dalam batas budget history,
Preview Undo memulihkan seluruh batch dengan tag yang sama. No-op/cold/stale/
omission/reorder/duplicate/forged/cap overflow menolak tanpa redo/checkpoint baru.

App mengizinkan Preview/Split + guard navigasi lama, Save/Export tetap boleh;
Source mode tidak membuka action atau menghangatkan model. UI action getter
menggunakan projection cached setelah guard; Check/Clear fixed width 112 dengan
label 14 px agar tidak wrap/menambah tinggi toolbar. Source/query focus, search,
cache, checkpoint dan history frontier tetap jalur lama. Native main Preview
input tidak dibuka. [Kontrak dan bukti Stage 6e](testing-stage6e.md).

GUI regression default tetap seluruh 116 checks/180s; batch berjalan dalam
invokasi Xvfb/temporary environment terpisah dengan alarm 180s sendiri, evidence
subfolder dan exit alami. Ini ekspansi suite, bukan relaksasi deadline lama.

## Invariant dokumen

1. `Document::content()` adalah pinjaman read-only dari buffer native yang sama dengan sumber preview, save, dan export.
2. Setiap perubahan teks efektif, termasuk undo/redo, menaikkan revisi monotonik. Edit no-op dan navigasi/seleksi tidak menaikkannya.
3. Dirty diturunkan dari `current_state_id != saved_state_id`. Edit baru selalu mengalokasikan state ID baru; undo/redo mengunjungi ID lama tanpa mengurangi revisi operasi. Kesamaan teks secara kebetulan bukan kesamaan checkpoint.
4. `replace` menaikkan ID dokumen, mereset revisi/history/checkpoint, dan mengganti buffer/path sekaligus. Dokumen pengganti tidak dapat dianggap tersimpan oleh hasil operasi dokumen lama yang kebetulan memiliki nomor revisi sama.
5. `mark_saved` menolak snapshot dokumen lain, revisi/state ID masa depan, dan revisi lebih tua dari yang sudah tercatat tersimpan. Revisi tersimpan hanya guard urutan acknowledgment; bukan lagi sumber dirty.
6. Save snapshot lama yang masih valid boleh memperbarui path (penting untuk Save As), tetapi state selain checkpoint yang ditulis tetap dirty. Undo saat save berjalan tidak boleh menyebabkan dokumen lama keliru clean.

## History, cursor, dan pencarian

Buffer aktif tetap `text_editor::Content`; history menyimpan snapshot teks penuh, bukan buffer aktif kedua atau AST. Budget gabungan 200 record / 8 MiB **panjang teks yang ditahan**, tidak termasuk overhead native, kapasitas allocator, metadata, atau salinan sementara. Trim membuang front stack; record terlalu besar tidak disimpan, sehingga undo/redo bisa hilang. Grouping insert dan deletion single-scalar tersedia; penggabungan deletion multi-scalar dan optimasi file besar belum ada.

Iced 0.13 tidak menyediakan setter caret/anchor seleksi publik. Rekonstruksi menggunakan Paste suffix → DocumentStart → Paste prefix, lalu memeriksa kesamaan teks raw dan posisi line/byte sebelum swap. Ini menghindari asumsi gerak visual Right = satu byte/grapheme dan mempertahankan empty line di akhir. Implementasi ini diuji melalui Cargo dan jalur clipboard/Go to Line/Find pada GUI X11 virtual; tidak membuktikan viewport atau desktop Wayland/Hyprland. Setiap Paste memakai `paste_exact`: menambah satu spasi sementara lalu Backspace untuk melindungi tab akhir dari `insert_at` cosmic-text 0.12.1. Aksi internal tidak melewati history Document satu per satu. History memulihkan caret, bukan seleksi. Transformasi source boleh mengakhiri seleksi; tidak ada akses private-layout widget.

`Content::text()` mempertahankan konvensi serializer LF aplikasi sebelumnya; history/search memakai `lines().join("\n")` agar tidak mengedit newline sintetis serializer. Kolom status/Tab dihitung berdasarkan Unicode scalar; bukan ukuran visual grapheme atau lebar tab/CJK. Word count memakai `split_whitespace`, bukan analisis linguistik/AST.

Semua formatter melewati `Document::transform_selection` → validasi transaksi → `commit`. Penolakan/no-op menjaga caret asli dan tidak membuat history/revisi atau membuang redo (seleksi tetap dikolapskan). Koordinator `transform_source` memusatkan guard, status, refresh search, dan refocus bagi toolbar maupun shortcut.

Block formatter mempertahankan indentasi dan separator tambahan pada body; hanya satu separator setelah prefix yang dikonsumsi. Normalisasi hanya pada keluarga prefix yang dipilih (heading, list, quote), bukan parse konteks dokumen atau container. Checkbox task dikenali pada awal body setelah separator; whitespace tambahan sebelum checkbox dapat membuatnya dianggap teks literal. Pilihan style yang sama pada seluruh baris nonblank men-toggle prefix lepas; seleksi campuran dinormalisasi. Ordered list dimulai dari 1 pada baris nonblank. Konversi task ke list biasa melepas checkbox. Blank selection hanya memberi prefix pada baris pertama. Caret block diletakkan di akhir rentang hasil.

Link meng-escape bracket/backslash label, menolak control character/multiline, dan memberi stub `https://` dengan caret sebelum penutup. Fenced block memakai rentang baris bersama dan fence backtick lebih panjang dari seluruh run di payload; caret berada pada endpoint seleksi semula di dalam body. Insertion ini tidak mengubah konteks AST di luar rentang, tidak mendeteksi bahasa, dan tidak men-toggle fence/link yang sudah ada.

Match search diinvalidasi pada perubahan snapshot/query. Controller me-refresh sesaat sebelum replace; replacement menolak range/slice yang tidak cocok. Replace All memakai match asli non-overlap sekali (tidak rekursif), satu transaksi history. Pindah ke dokumen lain mereset match aktif. Hasil Find ditampilkan sebagai konteks teks, bukan highlight native.

## Model-based core workflow tests (4j)

`document/workflow_tests.rs` adalah child test module dari History, sehingga native Cargo test dan harness mandiri mengompilasi file produksi/test yang sama. Hanya `#[cfg(test)]`; tidak ada state runtime, file data, konfigurasi, dependency, thread atau worker baru. CI yang sudah menjalankan harness otomatis memasukkan regresi ini.

`Rig` menghubungkan API produksi History, search, navigasi dan transaksi, tetapi **bukan** implementasi Document/controller native. `Reference` menyimpan frame `Vec<char>`, caret scalar dan Vec stack undo/redo; UTF-8 slicing/delta verifier/Group produksi tidak digunakan oleh oracle. Generator mempunyai seed u64 tetap, operasi bounded, clock numerik eksplisit, dan kategori gesture diketahui dari input scalar. Oracle memakai kebijakan pengelompokan yang didokumentasikan (jenis/kelas, 750 ms, 5 s, 64 scalar), bukan menebak dari hasil production verifier. Navigasi/save capture/transaksi atomic memberi barrier eksplisit; undo/redo dibandingkan berdasarkan teks/caret/state ID. Semua checkpoint yang tersisa dibandingkan lagi saat drain undo/redo akhir.

64 trace × 512 operasi = 32.768 langkah awal, plus restore saat drain. Cakupan termasuk insert/delete/paste/replacement/line ops/Enter/indent/navigasi/Find/Replace All/capture boundary/no-op/read-only tick serta clock mundur. Input dibatasi bila tumbuh >128 scalar agar generator tidak menjadi benchmark atau membuat dokumen eksponensial. Test model memakai limit history besar agar trimming tidak menyamarkan perbandingan stack. Untuk planner transaksi, rig dan oracle menerima TextChange produksi yang sama (diterapkan sebagai byte range vs scalar splice): ini tes konsistensi komposisi/history, bukan oracle independen kedua bagi semantik setiap formatter.

Budget test terpisah menjalankan 16 pasangan limit entri/byte × 512 operasi = 8.192 langkah. Memeriksa gabungan stack dalam budget, caret record valid, immediate peek/restore, dan grup tidak kehilangan base yang masih dirujuk. Tidak menyalin kebijakan trim sebagai oracle; bukan pembuktian urutan trim optimal. Tes eksplisit checkpoint gabungan dan 192 pasangan kandidat malformed/no-op melengkapi dua trace generator itu. Pemeriksaan search memakai pencocokan scalar non-overlap independen untuk count dan validasi slice target aktif.

Snapshot/revisi dan capture state di rig hanya stimulus untuk History/cache; tidak memanggil mark_saved, worker filesystem atau widget. Tidak membuktikan native grouping barrier, save acknowledgment, loader/serialisasi, code-context parser, pemulihan selection/caret/viewport atau guard UI. Tes native yang sudah ditulis dan build/Clippy aplikasi lengkap tetap belum dapat digantikan oleh hasil harness ini.

## Operasi baris (4h)

`line_edit::edit` memakai `text_edit::line_range` untuk extent baris logis yang sama dengan indent/block formatter. Seleksi yang berhenti pada kolom nol baris berikutnya mengecualikan baris berikutnya. Input/rentang tidak valid ditolak; tidak ada gerakan visual widget untuk menghitung isi baris. Ini operasi teks literal, bukan normalisasi list/AST; tidak menyalin ke clipboard atau merapikan Markdown.

Duplicate menghasilkan insertion-only `TextChange` di bawah blok. Jika ada LF sesudah blok, sisipkan body+LF setelah separator tersebut; di EOF tanpa LF, sisipkan LF+body. Ini menjaga terminal LF nonempty dan tidak membuat salinan kosong ekstra pada Select All yang berakhir di kolom nol. Baris EOF kosong memang diduplikasi dengan satu LF tambahan. Caret collapsed mempertahankan byte-column yang valid dari range awal, sedangkan nonempty selection menempatkan caret di awal blok salinan. Tidak memulihkan anchor seleksi.

Delete memakai range blok+LF berikutnya jika ada; jika blok berada di EOF tanpa LF berikutnya dan bukan baris pertama, range mencakup LF sebelumnya. Caret di replacement kosong (awal baris berikutnya atau akhir baris sebelumnya). Delete pada dokumen kosong mengembalikan None. Prefix/suffix di luar transaksi tidak berubah. Semua batas berasal dari LF ASCII dan batas scalar UTF-8 yang divalidasi, termasuk pemotongan LF sebelum blok EOF.

`Message::EditLines(Action)` diarahkan ke controller `transform_source` → `Document::edit_lines` → `transform_selection`/commit. Source-local Ctrl+Shift+D/K berbeda dari Ctrl+K link dan tidak mengambil input query/AltGr/logo. Toolbar memakai guard yang sama. Setiap aksi efektif atomic dan memisahkan typing/deletion group, refresh Find, fresh revision/state ID, serta checkpoint save lama tetap dapat dicapai selama budget history menyimpannya. No-op tidak membuang redo atau membuat dirty/revisi; seperti formatter lain, selection dapat dikolapskan dan caret dipulihkan, bukan anchor.

`Document::new` juga diperbaiki agar inisialisasi tidak membuang terminal empty line: buffer kosong → Paste terproteksi seluruh source → DocumentStart, sebelum state/history tracking dimulai. `replace` memakai initializer yang sama. Ini API publik yang sama dengan rekonstruksi caret, tanpa private-layout access. Tidak memakai `with_text(source)` yang kehilangan empty line terakhir. Tes load/replace/caret/clean/history dan regresi tab akhir sudah lulus melalui Cargo, bukan desktop. Tetap tidak mengklaim preservasi line ending CRLF verbatim; model source memakai baris LF.

Harness menjalankan 15 regresi operasi/rentang/EOF/matriks UTF-8. Tes Document/controller/binding sudah lulus dalam suite Cargo. Operasi dan rekonstruksi masih menyalin/scan O(n); tidak ada delta history, indeks baris incremental, atau jaminan latensi/RAM baru.

## Move Lines (4i)

`line_edit::Action::MoveUp/MoveDown` memakai extent blok yang sama dengan operasi 4h, lalu menukar body blok dengan tepat satu adjacent logical line. Range transaksi hanya meliputi kedua bagian dan LF pemisah; replacement mempunyai panjang byte yang sama. Prefix/suffix di luar transaksi tidak berubah. Tidak membuat AST, mengganti algoritme native key movement, atau menormalisasi isi line.

Collapsed caret memakai byte-column yang sama; selection nonempty menaruh caret pada awal blok pindahan dan mengakhiri selection. Anchor tidak disimpan/dipulihkan, sehingga command berulang sesudah blok berpindah akan memakai baris caret saja kecuali blok dipilih ulang. Undo menyimpan caret asli. Menggunakan byte offsets UTF-8, bukan lebar visual font, grapheme, tab stop, atau baris soft-wrap.

Kebijakan LF bersifat konservatif: terminal empty line sesudah LF terakhir adalah sentinel yang tidak bisa dipindah maupun dijadikan target; blank line sebelum sentinel tetap bisa dipindah. Source tanpa terminal LF juga tidak boleh menjadi ber-LF hanya karena blank line berpindah ke EOF. Jika swap menyentuh EOF dan status LF akhirnya berbeda, planner menolak transaksi, bukan membuang empty line atau menambahkan separator lain. Boundary tanpa tetangga dan replacement identik juga None. No-op menjaga caret/snapshot/redo lewat jalur transform_selection yang sama; selection bisa dikolapskan. Perpindahan efektif mempertahankan panjang, jumlah LF, status terminal LF, dan multiset line contents.

Source binding menangkap hanya focused bare `Modifiers::ALT` + ArrowUp/ArrowDown sebelum fallback native; Ctrl+Alt, Shift+Alt, logo, key lain dan query widget tidak ditangkap sebagai Move Lines. Global shortcut tetap menolak Alt. Toolbar memakai `EditLines` dan guard controller yang sama. Status, commit atomic/revisi/state ID, refresh Find dan batas history memakai jalur 4h, tanpa message/dependensi/config baru.

14 regresi core baru memeriksa swap/kolom, selection end-zero, EOF/sentinel/blank, no-op identik, round-trip caret Unicode, dan model referensi Vec-line permutation untuk seluruh rentang UTF-8 valid pada fixture. Matriks ini juga menguji panjang/LF/style/prefix/suffix tetap sama. Tes model/controller/binding sudah lulus dalam suite Cargo; tidak ada hasil GUI/IME/RTL/compositor/fokus/viewport yang dinyatakan lulus.

## Go to Line (4f)

`navigation.rs` tidak mengimpor Iced: menyimpan state panel dan mengubah input `line[:column]` menjadi offset byte UTF-8. Angka positif ASCII 1-based, whitespace luar diabaikan, zero padding diterima; input malformed/overflow/>64 byte dan koordinat di luar rentang ditolak. Kolom memakai Unicode scalar, termasuk tab/combining/ZWJ sebagai scalar tersendiri; endpoint setelah scalar terakhir valid. `split('\n')` mempertahankan baris kosong terakhir dan menyediakan satu baris bagi dokumen kosong. Semua offset berasal dari `char_indices`, bukan asumsi lebar visual. Resolve linear dan tidak membuat indeks/copy tiap baris.

`app/navigation.rs` memeriksa guard prompt/Open untuk membuka, mengubah input, atau submit. Save/ekspor biasa tidak memblokir navigasi. Panel terbuka dari Preview mengubah mode ke Source; Find dan Go to Line saling menutup tanpa menghapus query Find. Open mengisi alamat dari cursor/status column saat ini dan memfokuskan text input khusus. Submit menyelesaikan alamat terhadap buffer terbaru, memanggil `Document::set_caret`, menutup panel dan meminta fokus Source hanya bila rekonstruksi berhasil. Input invalid mempertahankan caret/selection/native buffer; error terlihat di panel. Kegagalan rekonstruksi tetap mempertahankan buffer lama dan dilaporkan lewat status. Escape menutup panel setelah prioritas prompt/error, sebelum Zen. Preview menutup panel; penggantian dokumen mereset seluruh state dan submit ditolak jika panel sudah tertutup.

`set_caret` menutup typing group dan menaikkan render generation saat mengganti Content, tetapi tidak mengubah snapshot/state ID/revisi, dirty, history, atau redo. Navigasi berhasil menghapus match aktif melalui `Search::clear_current`, bukan menghapus cache/query; refresh dengan snapshot yang sama tidak membangkitkan kembali target Replace lama. Native selection dikolapskan setelah sukses. Rekonstruksi masih O(n), tanpa setter caret/ID publik pada editor Iced. Smoke GUI X11 membuktikan caret scalar Unicode, fokus input/Source setelah submit/invalid/Escape, serta navigasi Find; harness mandiri tetap hanya menguji resolver/state murni. Viewport/RTL/IME dan desktop Wayland/Hyprland belum diverifikasi.

Fokus Source menggunakan container tanpa padding tambahan ber-ID `hylite-source-focus` yang membungkus satu TextEditor. `source_focus::task` menjalankan operasi publik `scope` dan `Focusable::focus`, bukan cycling `focus_next`. Operasi ini tetap fokus bila editor sudah fokus; scope yang hilang (misalnya Preview) tidak menyentuh field lain. Jalur undo/redo, transformasi Source, Close Find, dan Close Go to Line memakai operasi yang sama. Guard yang mencegah refocus ketika panel masih terbuka pada jalur edit tetap dipertahankan. Tidak memakai akses private state, fork Iced atau perubahan posisi caret sebagai workaround fokus.

Gutter belum diimplementasikan. Layout visual dan scroll buffer di balik `text_editor::Content` tidak tersedia melalui API publik 0.13 yang dipakai aplikasi. Jangan menganggap nomor baris terpisah berdasarkan indeks teks dapat mengikuti soft-wrap, font/shaping, atau scrolling internal. Solusi gutter yang akurat perlu integrasi widget/layout tersendiri dan verifikasi desktop, bukan menebak dari jumlah karakter.

## Enter continuation (4e)

Binding lokal menangkap hanya Enter tanpa modifier pada editor focused, lalu mengirim `InsertNewline`. Shift+Enter tetap aksi native. Controller memakai guard prompt/Open/Preview yang sama, membaca `editor.auto_continue_lists` (default true, startup-only), dan me-refresh Find setelah perubahan efektif; tidak memindahkan fokus ke widget lain.

`continuation::enter(text, caret)` menghasilkan `TextChange` murni atau menolak ke native Enter. Cakupan: list unquoted dengan 0–3 spasi awal, bullet/nomor/task sederhana, separator dan zero padding dipertahankan, nomor maksimum 9 digit, checkbox baru unchecked. Midline split mempertahankan suffix; empty item ASCII-blank melepas prefix di tempat tanpa LF baru dan mempertahankan indentasi. NBSP bukan blank yang boleh dibuang. Caret/rentang divalidasi UTF-8; baris >16 KiB tidak diteruskan otomatis. Tidak ada renumber sibling, general paragraph auto-indent, atau pengelolaan nested-list/outdent struktural.

Scanner fence ringan mengenali bentuk umum root/quote/list (termasuk marker list berulang); closer tidak boleh melepas marker list dari payload. Ini bukan parser container lengkap, sehingga false suppression konservatif masih mungkin. Sebelum commit kandidat yang lolos, `document/code_context` memakai `pulldown_cmark::Parser` yang sudah menjadi dependensi untuk memeriksa rentang code block, termasuk container, indented code, dan EOF fence tak tertutup. Cek terakhir ini mencegah lexical false closer mengizinkan edit otomatis di dalam code. Tidak memakai atau bergantung pada state highlighter. Parser transient saja, tidak ada AST dokumen persisten atau incremental. Tes konteks parser sudah lulus dalam suite Cargo.

`Document::insert_newline` menutup typing group dan memakai commit/rekonstruksi terverifikasi yang sama, satu record/revisi untuk satu aksi efektif. Opt-out, selection, unsupported context, code block, atau rekonstruksi gagal memakai `Edit::Enter` native, bukan menelan input. Undo/redo dan dirty tetap mengikuti checkpoint state ID; capture save tidak berubah. Snapshot, scan konteks, dan parse kandidat masih O(ukuran dokumen); batas baris bukan jaminan latensi/RAM seluruh aksi.

## Deletion coalescing (4g)

`Document::apply_at` memberi label kandidat `EditKind::Backspace` / `Delete` hanya bila tidak ada seleksi sebelum aksi native dijalankan. Selection replacement tetap `Atomic`, termasuk seleksi yang kebetulan hanya satu scalar. Perilaku Backspace/Delete pada `Content::perform` tidak diubah; tidak ada rekonstruksi hapusan, slicing atas asumsi grapheme, atau shortcut baru. Native action efektif tetap mengalokasikan revisi/state ID baru lewat jalur yang sama.

`History::deletion` memeriksa selisih panjang yang positif, batas UTF-8, range tepat di kiri/kanan caret lama, caret hasil, prefix/suffix identik, dan tepat satu scalar yang dihapus. Label tombol tidak cukup. Aksi multi-scalar/word/grapheme, perubahan nonlokal, caret yang berbeda, control/newline/tab dan ASCII punctuation selain underscore menolak coalescing dan dicatat atomic. Scalar Unicode yang lolos bukan klaim grapheme-aware; keputusan jumlah karakter yang dihapus tetap milik engine native.

`GroupKind` membedakan Insert/Backspace/Delete serta kelas Text/Whitespace, sehingga perubahan arah dan insert↔delete tidak merge. Validasi base record, kontinuitas state/caret, clock monotonik, idle <750 ms, durasi <5 s, cap 64 scalar, dan batas history lama dipakai bersama. Base terlalu besar atau terpangkas tidak boleh menghasilkan grup tanpa checkpoint. Metadata hanya bertambah jenis grup, bukan salinan teks tambahan. Budget gabungan tetap 200 record/8 MiB byte teks, bukan batas RAM/latensi seluruh editor.

Barrier navigation/selection/UI/window/save/undo/redo dari 4d berlaku pula untuk deletion. No-op di BOF/EOF menutup grup tanpa revisi/entry/redo loss. Capture save sebelum worker tetap memisahkan checkpoint yang dihapus sebelum/sesudah save, selama history budget menahannya. Guard controller dan refresh Find tidak berubah. Harness menjalankan 16 regresi deletion inti dengan waktu eksplisit; tes integrasi native/controller sudah lulus dalam suite Cargo.

## Typing coalescing (4d)

`History::record_edit` menerima state sebelum/teks sesudah/caret/jenis edit/waktu. Untuk insert, hanya delta satu `Insert(char)` yang valid secara UTF-8 dan tepat sama dengan prefix + karakter + suffix yang boleh merge. Tidak menebak dari label tombol saja. Seleksi, Paste/Enter dan formatter menjadi atomic. Backspace/Delete kandidat 4g tidak pernah merge dengan insert. Kelas teks dan whitespace terpisah; ASCII punctuation selain underscore/control tidak eligible. Tidak ada segmentasi kata/grapheme linguistik.

Metadata grup menyimpan ID base/end, caret berikutnya, jenis/arah dan kelas, clock awal/terakhir, dan jumlah scalar; teks ekstra tidak disimpan dalam metadata. Merge mensyaratkan undo base masih ada, state/caret tersambung, idle <750 ms, durasi <5 detik, dan jumlah sebelumnya <64. Clock mundur memulai grup baru, bukan panic. Batas history tetap 200 langkah/8 MiB byte record gabungan; tidak menyimpan record baru untuk merge. State sesudah edit yang oversized tidak dimerge, diberi warning limit; redo state besar bisa tidak tersedia.

`Document::apply_at` memungkinkan test memakai waktu eksplisit tanpa sleep. `apply` produksi memakai `Instant::now()`. Semua edit efektif tetap menaikkan revisi dan mengalokasikan state ID baru, walaupun history tidak menambah entri. Undo/redo memulihkan ID checkpoint, tetapi revisi operasi terus naik. Snapshot/render_key getter tidak menutup grup, sehingga search refresh/redraw tidak memecah setiap karakter.

`Document::save_snapshot()` wajib dipakai untuk capture save asinkron: tutup grup SEBELUM mengambil snapshot worker. Getter `snapshot()` biasa tetap read-only dan tidak menjanjikan setiap state intermediate yang telah dimerge dapat di-undo satu per satu. Controller `start_operation(Save)` menggunakan API ini bahkan bila dipanggil tanpa routing message UI. `mark_saved` menutup grup hanya setelah acknowledgment lolos guard. Save biasa masih mengizinkan typing baru; base grup berikutnya adalah checkpoint yang dikirim ke worker. Tidak menyeberangi checkpoint tersimpan kecuali record dipangkas karena limit.

Routing message UI menutup grup sebelum command; reply FileOpened/FileSaved/HtmlExported dikecualikan agar reply stale tidak menambah efek samping grouping. Reply valid yang mengganti dokumen/menandai save diproses oleh API dokumen. Native navigasi/seleksi dan caret reconstruction menutup grup. Event window Unfocused menjadi `EndTypingGroup`, tanpa mengubah caret/dirty/revisi/status. Grup hanya metadata, jadi barrier/no-op tidak menghapus redo.

## Syntax highlighting (4c)

Source/Split/Zen memakai builder editor yang sama dan selalu tipe highlighter yang sama, termasuk saat `editor.syntax_highlighting = false`. Mode disabled mengembalikan iterator kosong sambil memperbarui frontier, sehingga Iced dapat mengganti atribut lama dengan atribut default; bukan sekadar mengembalikan `usize::MAX` dan meninggalkan warna lama. Tidak ada message mutasi dokumen, worker, atau parsing AST dalam highlighter.

Iced memberi logical line secara berurutan, lalu memanggil `change_line` ketika konten berubah. Lexer menyimpan state fence **sebelum** setiap baris. Rewind memotong suffix cache dan mengembalikan frontier ke indeks perubahan atau frontier yang belum diproses, mana yang lebih awal. Tidak melompati baris yang belum diketahui. Edit fence, insert/delete, serta rewind diuji dengan membandingkan replay ke lexer baru. State context menempati O(baris yang diproses); kapasitas Vec bisa menahan peak sejak reset. Tidak menyimpan salinan teks atau span seluruh dokumen.

`Document::render_key()` = `(document_id, buffer_generation)` terpisah dari `Snapshot`. Native edit/navigasi memakai invalidasi baris Iced. Penggantian `Content` oleh caret reconstruction/format/undo/redo menaikkan generation; New/Open menaikkan document ID. Settings highlighter mencakup key tersebut, enabled, dan mode palet terang/gelap. Perubahan settings mereset lexer; getter/render tidak menaikkan revisi, dirty, atau history. Save acknowledgment tidak mengganti render key. Ini mencegah reuse state fence/atribut lama setelah buffer baru yang kebetulan memiliki checkpoint atau teks sama.

Lexer mendukung subset Markdown single-line dan fence tingkat atas; bukan CommonMark parser/embedded-language highlighter. Spans memakai byte UTF-8 valid, tidak tumpang tindih, dan hanya berisi warna. Inline lookahead memakai budget bersama 4 × byte baris; baris >16 KiB tidak menghasilkan warna, tetapi tetap memproses transisi fence. Input tidak dipotong. Tidak ada rekursi parser/backtracking tak terbatas. Budget dan cache ini bukan batas memori total/latensi desktop; adapter/konfigurasi sudah diuji dalam suite Cargo, tetapi pipeline render desktop belum dijalankan.

## Invariant operasi

```text
request → guard busy/unsaved → snapshot + token → native dialog/worker
                                                    ↓
current document ← validasi token/jenis/document ID ← hasil
```

Tracker mengizinkan hasil save/ekspor dari revisi lebih lama pada dokumen yang sama, sebab pengguna boleh mengetik selama kedua operasi tersebut berjalan. Controller Open memeriksa kesamaan snapshot penuh sebelum mengganti buffer. Jangan menerapkan pemeriksaan kesamaan revisi secara seragam pada semua hasil: itu dapat membuat Save As kehilangan pembaruan path saat pengguna mengetik.

New/Open/Close tidak berjalan selama operasi lain aktif. Konfirmasi unsaved memblokir editing. Save-before-action hanya dilanjutkan jika save sukses dan dokumen benar-benar bersih. Error/cancel mempertahankan aksi tertunda sampai pengguna memutuskan lagi.

## Konfigurasi

Mode dan tema memakai enum, bukan string bebas. Default per field memungkinkan konfigurasi singkat. Normalisasi numerik/font mempertahankan opsi valid lain, sedangkan error deserialisasi menolak file secara keseluruhan. Hasil load selalu membawa daftar warning; UI menampilkan warning terpisah dari error file agar dismiss salah satunya tidak menghapus yang lain.

`font_family` custom disiapkan sekali saat startup. Nama disimpan selama proses untuk memenuhi `Font::with_name(&'static str)` Iced 0.13. Tidak ada alokasi nama font di jalur render dan belum ada live reload konfigurasi.

## Bukan bagian dari tahap ini

Belum ada AST dokumen, WYSIWYG, parsing AST incremental, history AST lintas mode WYSIWYG, pemantauan perubahan file eksternal, autosave, atau recovery. Buffer Iced masih menjadi model teks native. Serialisasi operasi di dalam aplikasi bukan penguncian lintas proses atau jaminan terhadap force-kill.

## Alignment kolom tabel root (Stage 6f)

`Table.delimiters` hanya terisi untuk root projected table, sesudah hasil lexical
sesuai alignment pulldown-cmark. Nested atau fallback cap lama tetap tanpa target.
Picker menawarkan tiga pilihan noncurrent per kolom dan membawa exact snapshot,
table range, column/count, token range, expected alignment. Document mencari hanya
anggota root cache warm; lexical planner tidak pernah cukup untuk otorisasi.

Perubahan hanya colon token terpilih, dengan dash count/pipes/whitespace/header/
body verbatim. Cold/stale/forged/no-op ditolak tanpa mutasi/history. Caret sesudah
token baru, cache/search terbarui, satu Atomic trusted `PreviewTable` edge yang
ikut top-only Preview history. Source edits tetap barrier, Save/Export checkpoint
semantics dan logical focus sama. Tidak membuka free text Preview/cell editing.

Suite `table-alignment` terpisah tidak menggantikan 116 regresi maupun 9 batch;
setiap invokasi masih private Xvfb / alarm 180s / step 5 menit. [Kontrak, hasil
aktual dan batas Stage 6f](testing-stage6f.md).

## Whole raw body row tabel root (Stage 6g)

`Table.body_lines` hanya diproyeksikan sesudah auth delimiter root 6f, exact
jumlah physical lines vs parsed body rows, dan row source containment. Semua body
harus pipe-leading, indent 0–3 dan LF-terminated. Nested, EOF tanpa LF, non-pipe
atau fallback tidak memperoleh row actions; alignment dapat tetap tersedia.

Picker `Table actions…` mempertahankan urutan alignment di depan dan ukuran bar
lama, lalu Duplicate/Delete per body row. Target membawa full snapshot/table/
row index/total rows/columns/line. Document menuntut cache warm root membership.
Duplicate menyisipkan raw line sesudahnya, Delete menghapus hanya line beserta LF;
caret pada awal duplikat/following line. Header/delimiter dan byte lain utuh.
Duplicate yang melampaui cap 128 row/512 sel ditolak tanpa prefix/redo side effect.

Commit tetap Atomic trusted `PreviewTable`, cache/search/revision/focus/checkpoint
jalur lama, Source edit tetap Preview-history barrier. Tidak membuka teks sel/
body Preview. Suite `table-rows` keempat tidak mengganti 116+9+9 gate lama, tetap
private Xvfb/alarm180s/step5min. [Kontrak dan hasil aktual Stage 6g](testing-stage6g.md).

## Raw Markdown cell draft (Stage 6h)

`Table.editable_cells` berisi trimmed raw payload ranges hanya untuk root table
kanonis/rectangular/edge-pipe yang cocok dengan real parsed cell source ranges.
No padded/extra/ragged/nested/fallback targets. Target menyertakan full snapshot,
table/row/column/dimensions/payload; Document menuntut exact warm membership saat
Open dan Apply. Payload maksimal16KiB byte, satu baris, unescaped pipe/control
selain tab/odd trailing slash sesudah trim ditolak. EOF tanpa LF dipertahankan.

Picker lama tetap berurutan alignment lalu row, cell choices ditambahkan di akhir.
Panel draft mengganti widget tree dan memfokuskan native text input. State draft
hanya UI string/target/error, tidak mengubah buffer/revision/checkpoint/cache.
Enter/Apply mengganti satu payload verbatim, outer spacing dan bytes lain tetap;
Cancel/Escape tanpa commit. No-op menutup, stale/invalid mempertahankan draft
bererror tanpa rebase. Satu Atomic `PreviewTable`, Source history barrier tetap.

Selama draft, gate dispatch mem-pause mutasi/navigasi/history/mode dan file command
baru termasuk New/Open/Close/Save/Export, sampai Apply/Cancel. Async worker replies
masih melewati token guard lama; Source snapshot yang disimpan tidak mencakup draft.
Apply/Cancel memulihkan logical input context dan mode, search hanya refresh saat
commit. Tidak ada free typing atau glyph caret mapping Preview. [Kontrak/tes/batas
Stage 6h](testing-stage6h.md). Suite GUI kelima privat tidak mengurangi 116+9+9+9
gate lama; tiap alarm180s/step5min dan budget job45min tetap.

## Cell draft traversal dan explicit Apply & Next (Stage 6i)

`preview_cell::neighbor` menghitung row-major bounded index, tidak mengautentikasi
snapshot/range. Document menuntut warm current root membership 6h sebelum mengambil
real neighbor dari `editable_cells`; tidak ada wrap lintas tabel/padding/row baru.
Previous/Next tidak mengubah Source/caret/revision/history. Controller menolak
modified/invalid draft tanpa membuang value; notice navigasi bukan validation
error sehingga valid Apply tetap tersedia.

Apply & Next meminta explicit commit. Target lama diautentikasi, candidate Source
hanya mengganti satu payload, lalu full parser projection dipreflight agar root
start/dimensions/next cell metadata cocok sebelum satu Atomic `PreviewTable`.
Cache candidate dipasang sesudah commit, next target distamp full snapshot baru.
No-op hanya berpindah tanpa revision/redo side effect; last-cell command menolak
sebelum commit. Ini pengecualian eager preflight terhadap lazy model biasa, bukan
incremental parsing atau speedup. Parse/reconstruction tetap O(n).

State draft kini memiliki `navigation_notice` terpisah dari error. Apply & Next
mempertahankan focus input draft, search refresh pada commit, guard modal/file/
history/checkpoint lama tetap. Cancel membuang hanya draft terkini, bukan rollback
Apply terdahulu. Enter/Apply-close dan Escape-Cancel lama tidak berubah. Title
memperlihatkan row/column saat ini. Panel top-aligned/fixed control rows mencegah
footer/error menggeser input/navigation, tanpa menyentuh main Preview bar. Suite
GUI keenam privat mempertahankan lima gate lama utuh. [Kontrak dan bukti 6i](testing-stage6i.md).

## Explicit blank body-row insertion (Stage 6j)

`preview_table_row::Action::{InsertBefore,InsertAfter}` reuse exact current
snapshot/root table/body-line authentication dari6g, kemudian insertion-only
planner pada line.start/line.end. Anchor wajib real LF-terminated pipe-leading
body row; header-only/noncanonical/nested/fallback/cold/stale ditolak. Growth cap
sama dengan Duplicate diperiksa sebelum allocation:128 row termasuk header,
12kolom,512sel. Row baru hanya indent0–3spasi anchor + pipe + `  |` per header
column + LF. Tidak menyalin content ragged row, tidak serialize existing cells.

Satu Atomic PreviewTable edge menginvalidasi cache lama; fresh parser metadata
baru diperlukan untuk cell edit/navigasi berikutnya. Source caret di first blank
cell (`insertion.start + indent + 2`), search refresh/logical focus dan pending
Open/prompt/modal guards lama tetap. Save/Export token snapshot tidak berubah;
Source typing sesudah insert tetap history barrier sendiri. Tidak ada auto-row
creation pada Next atau sesi draft. Table picker menambahkan kedua action per
body row **setelah semua opsi lama**, mempertahankan labels/index/menu bar height
lama. Native suite ketujuh privat tidak mengganti enam gate terdahulu. [Kontrak,
tes dan batas6j](testing-stage6j.md). Reconstruction/history O(n), bukan speedup.

## Explicit atomic blank columns (Stage 6k)

`preview_table_column::{Target,Action,can_insert,full_lines,plan}` memindai outer
pipe separators dengan backslash parity. Hanya canonical rectangular root cells
6h ditambah seluruh physical lines termasuk delimiter pipe-leading/trailing LF.
`Table.insertable_columns` derived eligibility: editable cells, valid full lines,
growth<=12kolom/128row/512sel. Header-only boleh; ragged/nested/EOF/fallback tidak.
Metadata bukan authority Source. Picker menambah new column choices sesudah seluruh
prefix6j, memakai existing column index/header payload/marker/full snapshot.

Document menuntut warm current exact root range/dimensions/header/marker, lalu
planner hanya menyisipkan `  |` per header/body dan ` --- |` pada delimiter.
Seluruh old segments/prefix/suffix/tab/dash/alignment dipertahankan raw; tidak ada
AST serialization. Satu whole-table TextChange +candidate full Source preflight
membuktikan complete canonical root/dimensions sebelum satu Atomic PreviewTable.
Commit menginvalidasi projection lama; target berikut harus didapat dari parse
baru. Source caret di antara dua spasi new **header** cell, bukan body. Native
Source typing berikut tetap transaction/barrier terpisah; Save/Export snapshots,
modal/prompt/Open guard dan logical search/Go-to context tetap.

Scanner dibatasi column/line counts, tetapi reconstruction/history dan full parse
preflight tetap O(n), bukan speedup. Tidak ada column delete/reorder/resize atau
implicit insertion/inline editing. Suite GUI kedelapan private mempertahankan
ketujuh gate lama. [Kontrak, hasil dan batas6k](testing-stage6k.md).

## Explicit atomic column deletion (Stage 6l)

`preview_table_column::Action::Delete` memotong raw selected segment beserta
right separator per physical line sambil mempertahankan left separator/semua
retained bytes/alignment/tab. Scanner/backslash parity/canonical rectangle/full
LF lines6k tetap. `can_delete` menolak last column dan shapes di luar renderer
caps, bukan growth cap. `Table.deletable_columns` terpisah dari insertable flag:
12kolom/512sel boleh berkurang tanpa mengizinkan additional growth.

Document memilih eligibility sesuai action, tetap authentic warm current root/
full snapshot/ranges/header/marker; candidate full parse sebelum mutation harus
complete canonical root, same rows, one fewer columns. Satu whole-table Atomic
PreviewTable edge, old cache invalidated, fresh target untuk aksi/cell berikut.
Planner resolve Source caret ke header payload neighbor kanan (atau kiri jika
last selected) dari canonical replacement cells. Empty retained payload memakai
empty range. Source typing berikutnya transaksi/barrier sendiri; file/modal/
checkpoint/search/logical navigation context guards6k tetap. Tidak ada implicit
commit atau pembuangan draft. Delete explicit seperti Row Delete6g, tanpa prompt
baru; Undo memulihkan selected content. Delete last column tidak ditawarkan dan command-nya ditolak.

Delete menu tail sesudah seluruh prefix6k, termasuk column insert choices; tidak
mengubah width/font/action-bar geometry lama. Suite GUI kesembilan private
mempertahankan delapan gate lama. [Kontrak, bukti dan batas6l](testing-stage6l.md).
Reconstruction/history +preflight tetap O(n), bukan speedup. Bukan column reorder/
resize/bulk deletion/whole-table delete atau inline WYSIWYG.

## Bounded raw body-row movement (Stage 6m)

`preview_table_row::Action::{MoveUp,MoveDown}` memakai `neighbor` sebagai bounded
body-only index calculation; helper bukan provenance. Header/delimiter tidak
pernah neighbor, last/first body tidak wrap. Existing warm root/full snapshot/
line/dimensions checks Document6g tetap, kemudian exact raw selected/adjacent LF
lines ditukar dalam satu two-line TextChange. Ragged body/indent/tab/UTF-8/raw
markup/escape ikut line aslinya; header/delimiter/other rows/prefix/suffix utuh.
Growth room tidak diperlukan, renderer caps tetap. Identical raw lines no-op;
render-equal tetapi byte-different rows masih effective move.

Move-only candidate full Source parser preflight sebelum commit menuntut root
range/dimensions/body lines sama, lalu satu Atomic PreviewTable dan cache lama
diinvalidasi. Source caret awal selected physical row di posisi baru (Move up
relative0, Move down relative adjacent byte length), bukan payload/glyph mapping.
Native typing berikut menjadi history edge/barrier sendiri; file/modal/checkpoint/
search/logical Find/Go-to context sama. Refused/no-op Document tidak mengubah
Source/revision/redo/caret, tanpa implicit draft Apply. Physical focus setelah
no-op dan active query/Go-to masih manual.

Picker menambahkan bounded Move up/down **setelah seluruh prefix6l**. Native
suite kesepuluh private mempertahankan sembilan gate lama; tidak ada runtime
hooks/retry/refocus workaround. [Kontrak, bukti dan batas6m](testing-stage6m.md).
Full parse preflight/reconstruction/history O(n), bukan benchmark speedup. Bukan
header movement, arbitrary reorder/drag/selection atau inline WYSIWYG.

## Bounded raw column movement with alignment (Stage 6n)

`preview_table_column::Action::{MoveLeft,MoveRight}` dan `neighbor` bounded index
helper reuse full canonical column eligibility6k/6l (same as Delete, bukan growth
flag). Single-column/left edge/right edge tidak wrap; header-only/growth caps
boleh. Target tetap full snapshot/root dimensions/header/marker, authentication
warm current root di Document wajib; neighbor/scanner bukan provenance.

Per physical line, dua adjacent raw segments **termasuk right separator** ditukar.
Header/body whitespace/markup/Unicode/escaped pipes/entity ikut cell asal; raw
delimiter/dash/alignment ikut logical column, bukan slot lama. Row indent/trailing
bytes/kolom lain/prefix/suffix tetap. Satu whole-table TextChange, full Source
parser preflight same root/dimensions/canonical cells sebelum satu Atomic
PreviewTable. Entire raw column equality no-op sebelum mutation; body/alignment/
spacing differences tetap effective. Old cache invalidated pada commit dan target
berikut harus fresh. Source caret selected header payload di new column index,
bukan body/neighbor/glyph position. Modal/file/checkpoint/search/logical navigation
context guard sama; native typing berikut menjadi edge/barrier sendiri.

Picker append bounded column moves **setelah entire prefix6m** termasuk row moves.
Suite GUI kesebelas private mempertahankan sepuluh gate lama, no runtime hooks/
retry/refocus workaround. [Kontrak, bukti dan batas6n](testing-stage6n.md). No-op/
query/Go-to physical focus masih manual. Reconstruction/history +full parse O(n),
bukan speedup. Bukan drag/bulk/resize/selection atau general inline WYSIWYG.

## Bounded raw column duplication (Stage 6o)

`preview_table_column::Action::Duplicate` reuse `can_insert` growth caps dan
canonical outer-piped LF scanner6k. Selected raw segment beserta right separator
disalin tepat setelah dirinya pada header/delimiter/semua body. Alignment/dash/
whitespace/markup/Unicode/escaped pipes/entity disalin byte-exact; indent/trailing
bytes/neighbor/prefix/suffix tidak berubah. Single/empty/header-only/identical
payload boleh dan tetap effective;128row×3kolom→128×4 muat,128×4→128×5 ditolak.

Document authentication memakai exact warm root snapshot/dimensions/header/marker
dan `insertable_columns`, bukan lexical scanner sebagai provenance. Full candidate
Source parser preflight same complete root/same rows/columns+1/canonical cells dan
delimiters sebelum satu Atomic PreviewTable commit. Refusal preserve redo/cache/
checkpoint/caret; commit invalidate old projection. Copied header payload caret
resolved dari replacement, existing Split focus/modal/file/history/search handler
tetap. Source typing berikut transaksi/barrier sendiri; copied body cell Apply
butuh fresh target dan tidak mengubah original payload. No implicit Apply.

Picker append Duplicate per real column **setelah entire prefix6n**; tidak
memindah label/index lama. Suite GUI privat kedua belas mempertahankan sebelas
gate lama tanpa retry/runtime hooks/refocus workaround. Physical query/Go-to focus,
middle/3+kolom/header-only/empty/growth-bound UI dan compositor target tetap manual.
Full parse/reconstruction/history O(n), bukan speedup. Bukan drag/bulk/resize atau
inline WYSIWYG. [Kontrak, bukti dan batas6o](testing-stage6o.md).

## Backward atomic cell-draft transitions (Stage 6p)

`ApplyPreviewCellPrevious` ada di draft message whitelist dan shared controller
transition pipeline parametrized Direction. Document mempertahankan API
`edit_preview_cell_and_next` dan menambah `edit_preview_cell_and_previous`, keduanya
memakai private `edit_preview_cell_and_adjacent`. Current exact warm root payload
membership diautentikasi sebelum bounded neighbor/valid value. Candidate full
Source parse dan real previous payload provenance diselesaikan sebelum satu
PreviewTable commit; candidate cache dipasang dan target di-restamp Snapshot baru.
Unchanged transition hanya memakai current authenticated projection, tidak
mengubah Source/revision/cache/caret/checkpoint/history/redo. First-cell baik
changed/unchanged tidak diterapkan. Header/body row-major, no wrap/creation.

Caret tetap applied payload end, bukan previous payload. Cancel hanya membuang
draft saat ini; prior Apply tetap independent Atomic edge. Active modal/file/
Source/history/native typing/search/save guards tetap; physical query/Go-to focus
masih manual. Eligibility mengikuti raw cell6h (termasuk EOF), bukan full-line
column6k. Source authority dan history budget tidak ditambah.

UI append fourth navigation button160×32 setelah tiga kontrol6i tanpa menggeser
koordinatnya atau main picker. Invalid/first-cell disabled, dirty valid boleh
explicit Apply; enable helper bukan root authentication. Disabled native click
bisa melepas input focus; probe tidak memakai workaround, melainkan Cancel dan
reopen workflow invalid lalu menilai Source. Suite privat ketiga belas menjaga
23 old exercise bodies AST-identical dan dua belas gate lama; tanpa retry/runtime
hooks. Full parse/reconstruction/history O(n), bukan speedup. Tidak ada general
inline WYSIWYG/selection/scroll sync. [Kontrak dan batas6p](testing-stage6p.md).

## Single-line root paragraph raw drafts (Stage 7a)

`preview_paragraph` hanya bounded lexical scanner/Target/plan, bukan parser atau
provenance. Eligibility payload raw nonempty unindented single physical LF line,
no control chars,16KiB byte cap. `Block::paragraph_payload` diisi hanya root
Kind::Paragraph depth0 dengan physical-line eligibility; nested/heading/literal/
multiline/EOF/indent tidak mendapat metadata. Action136×24 di tail existing bar,
Show block source Fill shrinks tetapi bar height dan heading/table geometry tetap.

Document mengautentikasi exact warm root/snapshot/block/payload lalu full candidate
Source parser preflight **complete Paragraph range** dan new canonical payload.
Structural conversion/split/absorption ditolak sebelum mutation. Satu payload
TextChange, final LF/prefix/suffix/neighbor untouched; satu PreviewParagraph atomic
edge yang ikut scoped Preview history tanpa melintasi Source barrier. Caret pada
applied raw payload end. Projection invalidated saat commit; target berikut fresh.
No-op closes authenticated draft tanpa Source/cache/revision/history/redo mutation.
Trailing spaces editable bagian payload, tidak ada AST serialization/normalization.

Separate paragraph Draft/message/controller/UI modal mutually exclusive dengan
cell draft; Source guard/title/view/whitelist/focus menanganinya. Existing file/
confirmation/history/mode commands blocked selama draft, async replies allowed
namun snapshot immutable dan draft tetap. Search refresh only effective Apply,
logical Find/Go-to context preserved. Source authority dan history200entries/8MiB
tidak ditambah. Cold/stale/forged target tidak rebase atau menghangatkan cache.

Suite GUI privat keempat belas menjaga24 old exercise bodies AST-identical dan
semua13 gate lama tanpa refocus/retry/runtime hooks/oracle changes. Full candidate
parse/reconstruction/history O(n), bukan speedup. Multiline/EOF/inline typing/
selection/glyph mapping/scroll sync dan desktop target tetap di luar scope.
[Kontrak, bukti dan batas7a](testing-stage7a.md).

## Bounded native multiline paragraph drafts (Stage 7b)

`preview_paragraph_multiline` memakai Target shape7a tetapi scanner/plan terpisah:
2–128 physical lines nonempty/unindented, total16KiB, LF final required, LF internal
boleh/no tabs/CR/other controls. `Block::multiline_paragraph_payload` hanya root
Paragraph eligible; single-line metadata/action/APIs tetap. New action136×24 tail
bar yang sebelumnya Source-only, height/spacing dan existing controls unchanged;
Source Fill shrinks pada multiline bar, bukan claim semua hit regions identik.

New modal local `text_editor::Content` terpisah dari Source/single/cell draft;
Enter native newline, Apply button eksplisit, Escape Cancel. Source helper
`buffer_text`/`paste_exact` direexport internally tanpa perubahan implementasi,
sehingga raw draft paste mempertahankan trailing LF/control untuk validation,
bukan invent/drop LF. Native insertion/paste cap preserves last accepted bytes/
selection; invalid intermediate draft editable. No draft-session Undo. Named zero-
padding Container scope +`source_focus::task_in` memastikan idempotent native draft
focus, tidak focus_next atau menyentuh Source/query. Parser rejection refocuses
draft; final Apply/Cancel restores existing logical context.

Document authenticate exact warm multiline root/snapshot/ranges, no cross-API auth,
then full candidate parse complete root Paragraph +canonical multiline payload.
No split/conversion/absorption/partial Apply; original final LF/prefix/suffix/
neighbors untouched. Existing PreviewParagraph atomic history kind/caps/Source
barriers reused; caret at last applied payload end. Unchanged close preserves
revision/cache/caret/checkpoint/history/redo. Async Save/Export immutable snapshots,
modal guards/search/logical Find/Go-to apply to new modal. No AST serialization.

Suite privat15 preserves25 old exercise bodies AST-identical and all14 old gates;
no runtime hooks/retry/oracle/refocus workaround/deadline changes. Full parse/
reconstruction/history O(n), not speedup. EOF/indent/blank-separated blocks,
inline WYSIWYG/selection/glyph mapping/sync scroll and target compositor remain
out of scope. [Contract/evidence/manual limits7b](testing-stage7b.md).

## Multiline draft-local snapshot history (Stage 7c)

`multiline_draft_history` pure bounded raw Record{text,caret}, without Document/
Snapshot/state_id/save/file/operation identity. Draft owns a fresh History per Open;
100records total Undo+Redo /1MiB retained raw text /record16KiB. One effective native
edit one step, no coalescing; no-op/selection/motion/refused large paste no record/
redo truncation. Branch clears draft redo; oldest Undo then farthest Redo eviction,
including unequal-size transfers. This is not total allocation/process RAM bound.

Record byte caret native (line,column UTF8 bytes); Source `content_at` reexported
internally with body unchanged preflights exact native reconstruction before
history transfer. Restore collapses selection at captured caret, does not restore
anchor/direction/scroll. Raw invalid intermediate/final LF/tab/Unicode retained;
lexical validation refreshed after effective restore, no structural/auth bypass at
Apply. Empty stack leaves parser error intact. Full snapshots/rebuild O(n), no
incremental/performance claim.

Focused `session_key_binding` wraps old7b base binding, converts exact Ctrl+Z/Y/
Shift+Z into separate `MultilineDraftHistory` custom binding. Even empty captured
stack does not leak global Preview history. Controller only permits this contextual
message inside multiline modal; Source Undo/Redo/PreviewHistory remain blocked,
single-line/cell modals unchanged. Enabled appended Undo draft/Redo draft controls
restore named scope focus, no focus_next; old Apply/Cancel geometry retained.
Unfocused keys and disabled-click focus are not promised history actions.

Cancel/Apply/no-op close drops entire local stack, reopen fresh. Local restoration
never Document mutation/cache/search/caret/revision/checkpoint/dirty/redo change;
full authenticated root Apply is still one PreviewParagraph edge. Save/Export
immutable/checkpoint and file/modal guards unchanged. Does supersede7b's explicit
no draft-session Undo limitation only for multiline native panel. New independent
private GUI suite16 preserves26old exercise bodies and all15gates. See
[contract/tests/evidence/manual limits7c](testing-stage7c.md).

## Unchanged-only multiline root draft navigation (Stage 7d)

`document::ParagraphDirection` +`adjacent_multiline_paragraph` memilih nearest
eligible root metadata dalam urutan Source setelah exact warm target auth7b.
Hanya root vector, tidak children: skip single-line/heading/nested/code/literal/
EOF/unsupported. No wrap/rebase/parse/warm; neighbor snapshot/range current.
Read-only, tidak mutate Source/cache/history/caret/revision/checkpoint/dirty/redo.

Controller only allows move in valid multiline Preview/Split modal, normal file/
pending/Open guards. Raw draft must equal authenticated current payload, error
must be None; dirty/invalid/stale/boundary retains accepted text/selection/history.
Separate notice doesn't invalidate Apply; effective local edit/history restore
clears it. Named focus scope after move/refusal, not focus_next. Native buffer and
neighbor affordances constructed before replacing old session; successful movement
resets local history/caret0, including old session redo after Undo to original.

Previous/Next160×32 append a new row after existing Apply/Cancel/Undo/Redo, those
coordinates/input/root bars unchanged; footer moves44px. No new shortcut or Apply
and Next, no implicit Source commit. Bounds/error disable controls; disabled click
blur isn't refocus proof. Apply explicit closes as one PreviewParagraph edge,
Cancel/no-op/navigation aren't Source history. Cell/single-line panels,7b grammar,
Source helpers/history, immutable Save/Export and logical search/Go-to unchanged.

Lookup/auth copies/root walk O(n), not performance claim; no glyph selection
mapping, cross-kind/EOF editing, block creation/sync scroll. Native Copy proves
raw value/focus, controller selection tests aren't native anchor/pixel proof.
[Contract/tests/evidence/manual limits7d](testing-stage7d.md), independent suite17;
27old exercise bodies and all16gates preserved.

## Two-phase multiline Apply & Next/Previous (Stage 7e–7f)

Opaque `MultilineMovePlan` (dahulu `MultilineNextPlan`) dari read-only
Document.prepare memuat anchor, original
payload/full Source, provisional next ranges/value/neighbor flags, optional native
Source Content/candidate text/full parsed model. Exact warm root auth7b+nearest next
7d, full candidate root preservation, byte-delta-shifted complete forward root/payload
unchanged, or — for Previous — an earlier root whose byte offsets cannot move and
must still re-verify exact block/payload/model text; UTF8 grow/shrink/canonical bounds checked. Native Source content_at
reconstruction verified at applied payload end; no mutation/barrier change at prepare.

Controller builds exact native next Draft before calling commit. Commit rechecks
warm anchor snapshot/ranges and full Source equality, then single native buffer swap
and shared record_edit(PreviewParagraph)/identity/caps/barriers, installs candidate
model warm, stamps next snapshot from actual commit. No predicted revision/state_id,
no fallible parse/next buffer initialization after Source changed. Opaque fields/
consumed plan prevent callers forging candidate; context read-only except caret/
Save ack can interleave, text/revision/replace/cold mismatch refuses unchanged.

Unchanged returns no-edit plan: fresh session but Source identity/caret/cache pointer/
checkpoint/redo/search unchanged. Changed opens native caret0/fresh local history;
Source caret remains previous applied payload end. Cancel next restores Source
focus there, does not Undo previous Apply. Each Apply its own atomic edge; local
empty Undo cannot reach Source. Dirty notice doesn't disable valid Apply & Next;
last-boundary/invalid/parser/stale/native-prefill refusal keeps draft/history, scope
focus restored. No wrap/implicit creation/Apply & Previous/new Apply shortcut.

160×32button appended to same Previous/Next row, old geometry unchanged. Existing
Apply/Move paths/7b grammar/single-line/cell/Source helpers unchanged. Immutable Save/
Export/search/logical Go-to/file/modal guards retained. Full buffers/copies/model/
parse/reconstruction O(n) and extra memory, not speedup/process-RAM improvement.
[Contract/tests/evidence/manual limits7e](testing-stage7e.md) and
[7f](testing-stage7f.md); suites18–19 independent, 28old exercise bodies and all18
pre-7f gates preserved. Apply & Previous uses the same opaque plan/commit path with
direction-parameterized neighbour offsets.

## Shortcut commit eksplisit draft multiline (Stage 7g)

`commands::draft_commit_shortcut` adalah satu-satunya pemetaan tombol commit dan
hanya menerima `Modifiers::CTRL` persis: Enter→Apply, PageUp→Apply & Previous,
PageDown→Apply & Next. AltGr (Ctrl+Alt), Logo compositor, Shift dan tanpa modifier
ditolak, jadi tidak ada commit implisit dan Enter/PageUp/PageDown native tetap
menyunting teks.

`session_key_binding` memanggil helper itu hanya untuk status *Focused* setelah
jalur history lokal7c, lalu membungkusnya sebagai `text_editor::Binding::Custom`.
Karena message-nya identik dengan tombol, seluruh guard7d–7f (error/stale/boundary/
unchanged/mode/modal/file operation) tetap berlaku tanpa jalur kedua; tidak ada
edge history, mutasi caret/checkpoint/cache, atau Apply implisit tambahan. Source
dan panel sel/paragraf satu baris tidak mendapat shortcut baru; tata letak tombol
tidak berubah, hanya teks bantuan footer.
[Kontrak/tests/evidence/manual limits7g](testing-stage7g.md); suite20 independen,
28old exercise bodies dan 19 gate sebelumnya preserved.

## Hardening properti draft multiline (Stage 7h)

Tanpa perubahan kode produk, `preview_multiline_property_tests` menambahkan xorshift
deterministik dan korpus blok untuk membandingkan setiap commit 7e/7f dengan
transformasi acuan `replace_range(payload, value)` pada Source asli: buffer native
identik, caret di akhir payload, hasil tetap satu root multiline dengan offset tepat,
neighbour mempertahankan payload, satu edge Undo/Redo, dan unchanged move tanpa
identitas/history berubah. Refusal (nilai tidak valid, plan basi) wajib
byte-identical termasuk pointer cache preview. History lokal diuji tetap dalam batas
16KiB/1MiB/100 record di bawah 300 operasi acak.

Temuan Iced 0.13: `text_input` menangkap `Enter`/`PageUp`/`PageDown`/`Escape` saat
fokus, jadi hanya draft `text_editor` yang dapat memakai shortcut commit7g; panel
paragraf satu-baris dan sel tetap mengandalkan `on_submit`. Batasan ini
didokumentasikan di helper agar tidak menjadi klaim fitur yang hilang.
[Kontrak/tests/evidence7h](testing-stage7h.md).

## Konteks draft dan umpan balik blokir (Stage 7i)

`Document::multiline_context`/`paragraph_line` bersifat baca-saja dan selalu
mengautentikasi target warm7b lebih dulu; offset UTF-8 diperiksa sebelum menghitung
rentang baris1-based serta indeks/jumlah root multiline eligible dari proyeksi warm.
Hasilnya disalin ke sesi draft saat dibuka/dipindah (`Draft::context` / `Draft::line`)
dan **tidak** dihitung ulang dari byte kandidat, sehingga header tidak pernah
mengklaim rentang yang belum ada di Source. Formatnya fungsi murni dan tinggi baris
tetap20px agar koordinat input/tombol tidak bergeser.

`app::Hylite::report_blocked_draft_action` memberi status ketika gesture pengguna
(Save/SaveAs/New/Open/Export/mode/Undo/Redo/Find/FindNext/GoTo/sidebar/Zen/close)
ditolak karena panel draft terbuka; ketikan, boundary mengetik, dan reply asinkron
tetap senyap. Guard, urutan whitelist, dan semantik blokir tidak berubah.
[Kontrak/tests/evidence7i](testing-stage7i.md).

## Navigasi dan Apply & Previous/Next paragraf satu baris (Stage 7j)

Panel7a hanya punya `Apply`/`Cancel`. Stage7j memberi paritas terbatas dengan draft
multiline7d–7f: `Previous paragraph`/`Next paragraph` memindahkan draft **hanya** bila
teks belum berubah (draft kotor → notice, draft dipertahankan), sedangkan
`Apply & Previous`/`Apply & Next` meng-commit payload sekarang lalu membuka payload raw
root tetangga sebagai draft fokus baru dalam satu langkah. Root eligible tetap
`Kind::Paragraph` dengan `paragraph_payload` dan tanpa `multiline_paragraph_payload`,
urutan Source, tanpa wrap, tanpa pembuatan paragraf.

`SingleMovePlan` menyimpan hasil fase **rencana baca-saja** (`prepare_single_paragraph_*`,
`&self`): kandidat Source, parse penuh, bukti payload hasil tetap satu root paragraf
satu baris dengan rentang byte tepat, blok tetangga di kandidat (offset digeser untuk
`Next`), byte payload tetangga, flag `[previous, next]`, provenance baris, dan native
buffer dengan caret di akhir payload hasil. Draft tetangga dibangun **dari plan** —
offset pasca-edit tidak pernah dibaca dari dokumen yang belum berubah. Fase commit
(`commit_single_paragraph_move`) memeriksa ulang payload anchor dan `buffer_text()`,
lalu mencatat tepat satu edge `EditKind::PreviewParagraph`; apply unchanged
(`edit: None`) tidak menambah edge dan mempertahankan redo serta snapshot/render
key/caret.

`decide_preview_paragraph_adjacent` memisahkan keputusan baca-saja (`AdjacentDecision`)
dari mutasi `&mut self` supaya borrow checker tidak memaksa dua fase bercampur.
Boundary memakai `navigation_notice` sehingga `Apply` tetap aktif; error lain memakai
`error`. Karena `text_input` Iced0.13 menangkap `Enter`/`Escape`/`PageUp`/`PageDown`
saat fokus, tidak ada shortcut baru — hanya dua baris tombol tambahan di bawah baris
`Apply`/`Cancel`, sehingga koordinat suite GUI lama tidak berubah.
[Kontrak/tests/evidence/manual limits7j](testing-stage7j.md); suite21 independen,
28old exercise bodies dan 20 gate sebelumnya preserved.

## Baris Source pada draft sel tabel (Stage 7k)

Panel sel7a hanya menyebut baris tabel. `Document::cell_line` kini memberi baris
Source1-based untuk payload sel, dengan otentikasi warm yang sama seperti edit sel:
`preview_cell_payload` lebih dulu memeriksa snapshot, blok tabel yang dirender,
`rows`/`columns`, dan rentang `payload` persis; setelah itu offset byte diperiksa
(`is_char_boundary`) sebelum menghitung `\n`+1. Target basi/forged/blok non-tabel
menghasilkan `None` dan panel kembali ke teks petunjuk lama.

`ui::cell_edit::Draft` mendapat `line: Option<usize>` yang diisi saat sesi dibuka
(`open_preview_cell`), dipindah (`move_preview_cell`), dan setelah
`apply_preview_cell_adjacent` (memakai target hasil transisi). Teks provenance
disalin sekali dan **tidak** dihitung ulang; Apply tetap menolak target basi, jadi
provenance tidak pernah menjadi otorisasi. Baris petunjuk tetap tinggi20px sehingga
input dan kedua baris tombol sel tidak bergeser — cukup teks yang berubah.
[Kontrak/tests/evidence/manual limits7k](testing-stage7k.md); tanpa suite ke-22,
21 suite lama dijalankan ulang utuh.

## History draft lokal panel satu baris (Stage 7l)

Panel7a/7j (paragraf satu baris) dan6h/6i/6p (sel tabel) tidak punya history lokal;
Stage7l memberi keduanya `multiline_draft_history::History` sendiri — 100 record
Undo+Redo /1MiB teks mentah tersimpan /≤16KiB per record — plus dua tombol eksplisit
`Undo draft`/`Redo draft`. Record tetap hanya `{text, caret}`: tanpa Snapshot,
`state_id`, identitas file, operasi, atau status simpan. Satu edit efektif = satu
langkah, tanpa coalescing waktu/caret; cabang baru membuang redo; eviksi Undo
tertua lalu Redo terjauh.

`app::paragraph_edit::history_preview_paragraph` dan
`app::cell_edit::history_preview_cell` mengambil target, menolak record melebihi
`MAX_BYTES`, lalu `History::step` dan **memvalidasi ulang** nilai hasil restore
(pesan error paragraf/sel yang sama seperti Apply). Restore efektif membersihkan
`navigation_notice`/error sebelumnya. Undo/Redo draft tidak pernah menyentuh
dokumen, snapshot, render key, caret Source, revisi, checkpoint, dirty flag, atau
stack Source/`PreviewHistory`; `Apply` tetap satu-satunya jalur ke Source dan tetap
mencatat tepat satu edge. `Cancel`/Apply/penutupan no-op membuang seluruh stack.

Tombol memakai `can_undo`/`can_redo` sehingga disabled tanpa state palsu, dan baris
tombol sel tetap lebar 90/130/120/120px agar tinggi baris serta koordinat suite
GUI lama tidak bergeser. Tidak ada shortcut keyboard baru (`text_input` Iced0.13
menangkap `Enter`/`Escape`/`PageUp`/`PageDown`), tanpa grouping, dan tanpa berbagi
history antar panel. [Kontrak/tests/evidence/manual limits7l](testing-stage7l.md);
suite ke-22 independen, 28 badan exercise lama dan 21 gate sebelumnya preserved.

## Keyboard Undo/Redo untuk panel draft satu baris (Stage 7m)

Panel7a/7j dan6h/6i/6p memakai `text_input`, yang tidak punya undo stack dan
melaporkan `Ctrl+Z` sebagai *captured* saat fokus. Stage7m karena itu memakai rute
**netral**: `commands::history_keys` mem-parse `Ctrl+Z`/`Ctrl+Y`/`Ctrl+Shift+Z`
dengan modifier tepat (AltGr, Logo, dan Shift tanpa Ctrl ditolak) menjadi
`Message::DraftHistory(redo)` dari subscription, lalu
`Hylite::draft_history` memutuskan tujuan berdasarkan `draft_history_panel()`:
panel paragraf → stack paragraf, panel sel → stack sel, tanpa panel →
`preview_undo_redo` (gerbang mode `Preview` yang sama seperti sebelumnya, sehingga
`Source`/`Split` tetap dijalankan binding widget Source dan tidak pernah terjadi
undo ganda).

`event::listen_with` hanya menerima *fn pointer*, jadi closure yang menangkap state
tidak mungkin; itu sebabnya pesan dibuat netral dan keputusan panel dipindah ke
controller. `preview_history_shortcut` tetap ada sebagai pembungkus bergerbang
`Status::Ignored` untuk binding `text_editor` multiline, dan panel multiline tetap
memblokir pesan netral. Snapshot memakai kembali `multiline_draft_history::History`
(100 record /1MiB /≤16KiB, satu edit efektif = satu langkah, tanpa grouping): tidak
ada state, file, thread, atau dependency baru, dan stack tetap hilang saat `Cancel`/
`Apply`/pindah draft. Footer kedua panel menyebut shortcut-nya tanpa menggeser
tombol, tinggi baris, atau koordinat suite lama.
[Kontrak/tests/evidence/manual limits7m](testing-stage7m.md); suite ke-23
independen, 28 badan exercise lama dan 22 gate sebelumnya preserved.

## Umpan balik batas history draft (Stage 7n)

Gestur history lokal yang tidak bisa bergerak tidak lagi mati senyap:
`history_preview_paragraph`/`history_preview_cell` mengisi `navigation_notice`
(`Nothing to undo/redo in this draft; Source is unchanged.`) ketika `History::target`
kosong, dan `That draft snapshot is too large to restore; draft kept.` ketika target
melebihi `MAX_BYTES` atau `step` menolak. Pesan memakai notice, bukan `error`, jadi
`Apply` dan validasinya tidak berubah; restore efektif dan ketikan berikutnya
membersihkan notice (panel paragraf sekarang konsisten dengan panel sel yang sudah
membersihkannya di `change_preview_cell`).

Batas yang dikunci: `Ctrl+PageUp`/`Ctrl+PageDown` tetap bukan rute commit untuk panel
satu baris karena oracle suite 7j menetapkannya; mengubahnya adalah keputusan produk
terpisah. Penguatan 7n bersifat test-only: RNG deterministik (LCG, tanpa dependency)
plus reference model yang menghitung ulang total byte (bukan menyalin counter
produksi) memverifikasi urutan eviksi, pembuangan redo, dan akuntansi byte pada 5 seed
× 400 operasi; dua test controller memastikan pesan/gestur history tidak pernah
menyentuh Source atau stack-nya. [Kontrak/tests/evidence/manual limits7n](testing-stage7n.md).

## Tab multi-buffer (Stage 8a)

`Hylite` kini memegang `tabs: Vec<app::tabs::Tab>` + `active_tab`. `Tab` hanya
`Option<Document>`: `None` menandai slot yang sedang aktif, karena `Document`-nya
hidup di `Hylite::document` supaya seluruh jalur edit yang sudah ada (Source, Find,
GoTo, draft sel/paragraf, preview history) memakai borrow yang sama persis.
Konsekuensinya pindah tab adalah dua `mem::replace`, bukan rebuild: content native,
caret/seleksi, history bounded, snapshot/checkpoint, path, dan dirty flag ikut
berpindah utuh, dan tidak ada edge history baru di Source mana pun.

Guard `tab_switch_allowed` menolak tab baru/pindah/tutup saat panel draft terbuka,
konfirmasi tertunda, atau operasi file aktif — draft terikat pada target di buffer
aktif sehingga target basi tidak pernah terjadi. Tab terakhir dilindungi dan tab
kotor tidak pernah dibuang diam-diam (keduanya lapor lewat status). Pindah tab
me-reset `GoTo` dan pencarian lewat `search.reset_document()` + `refresh_search()`
seperti `Open`, jadi match/target buffer sebelumnya tidak bocor.

Shortcut tab memakai `Ctrl+T`, `Ctrl+W`, dan `Alt+1..9` dengan modifier persis
(AltGr/`Ctrl+Alt` tidak pernah memindah buffer); `Ctrl+Tab` sengaja tidak dipakai
karena editor Source bisa menangkap Tab bermodifier, sehingga cycling tersedia lewat
tombol `›`. Strip tab dirender **di dalam baris toolbar yang sudah ada** (tinggi 28px,
lebar tetap 330px, disisipkan sebelum `horizontal_space()`), jadi setiap koordinat
editor — dan seluruh suite GUI lama — tidak bergeser. Bentuk judul jendela tidak
berubah agar oracle title lama tetap valid. [Kontrak/tests/evidence/manual limits8a](testing-stage8a.md).

## Workspace folder dan file tree (Stage 8b)

`src/workspace.rs` adalah modul murni `std` (ikut strict-core harness): memindai
satu folder menjadi `Tree`/`Entry` dengan batas eksplisit — `MAX_DEPTH = 8`,
`MAX_ENTRIES = 512`, hanya `md`/`markdown`/`mdx`/`txt`, nama berawalan titik
diabaikan, symlink tidak pernah diikuti (dihitung `skipped`), entry tak terbaca
dihitung lalu dilewati. Urutan selalu folder dulu lalu file, case-insensitive.
`Workspace` menyimpan `collapsed: BTreeSet<PathBuf>` dan `selected: usize`;
`rows()` menurunkan baris yang terlihat, `move_selection` meng-clamp tanpa wrap,
`activate` men-toggle folder atau mengembalikan baris dokumen.

Controller (`src/app/workspace.rs`) menjalankan scan di blocking pool dengan nomor
urut (`workspace_seq`), jadi hasil scan lama tidak bisa menimpa folder yang dipilih
belakangan. Tidak ada watcher: `F5`/tombol Refresh memicu scan ulang, dan state
`collapsed` serta path terpilih dipertahankan. Membuka baris dokumen memakai
`PendingAction::OpenAt(path)` — jalur `Open` yang sama (token operasi, snapshot
guard, konfirmasi unsaved changes), bukan jalur I/O kedua. Karena membawa
`PathBuf`, `PendingAction` tidak lagi `Copy`.

Sidebar hanya mengganti area isinya dengan `src/ui/workspace.rs`: header folder +
tombol Refresh, baris yang semuanya tombol nyata (folder → `ToggleWorkspaceDir`,
file → `OpenTreeFile`), footer hitungan dokumen/folder dan peringatan truncation.
Lebar sidebar dan koordinat editor tidak berubah. Keyboard: `Alt+J`/`Alt+K`/
`Alt+Enter` (Alt persis), dan `Ctrl+Shift+O` untuk memilih folder — `Ctrl+O` tetap
file tunggal. Launch argumen `hylite <folder>`/`<file>` masuk lewat
`Hylite::with_launch_path`, satu-satunya konstruktor.
[Kontrak/tests/evidence/batas 8b](testing-stage8b.md).

## Gambar read-only di Preview (Stage 8c)

`src/images.rs` (murni `std`, ikut strict-core) adalah satu-satunya pintu masuk
gambar. `resolve` menolak skema apa pun (`http:`, `https:`, `file:`, `data:`,
`//host`, `scheme:`) **sebelum** I/O, men-decode `%XX` (escape rusak ditolak),
dan me-resolve path relatif hanya terhadap folder dokumen — dokumen tanpa path
menghasilkan `MissingBase`, bukan tebakan working directory. `inspect` lalu
memeriksa `metadata` (≤ 4 MiB, harus file biasa) dan membaca maksimum 64 KiB
header untuk dimensi PNG/GIF/BMP/JPEG; `MAX_DIMENSION = 8192` dan
`MAX_PIXELS = 40 000 000` menjaga biaya dekode. Format lain → `UnsupportedFormat`.
Setiap kegagalan punya `label()` stabil yang menjadi teks marker inert.

Proyeksi preview tetap menghasilkan teks marker lama
(`[image not loaded: ![alt](src)]`) sehingga semua oracle lama byte-identik; run
itu juga membawa `images::Spec { src, alt }`. Renderer hanya mengganti **satu**
run gambar di dalam paragraf yang isinya tepat itu menjadi widget gambar
(`container(image).max_width(560).max_height(420)` + tooltip `alt · W×H`).
Gambar yang bercampur teks, berada di tabel, atau di dalam sel tetap marker.
`render_preview` menerima `base_dir` dari `Document::path().parent()`; tidak ada
renderer yang boleh mengarang path.
[Kontrak/tests/evidence/batas 8c](testing-stage8c.md).
