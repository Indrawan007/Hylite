# Arsitektur setelah iterasi Source 4a–4j

Build debug, Clippy dan **320 tes Cargo headless lulus** pada CI Ubuntu 24.04 / Rust stable; lihat [bukti dan batas verifikasi](verification-native.md). Smoke GUI terpisah pada X11/Xvfb + tiny-skia juga lulus; lihat [cakupannya](testing-gui-x11.md). Ini bukan kelulusan desktop Wayland/Arch/Hyprland.

Perluasan [lifecycle X11](testing-lifecycle-x11.md) membuktikan New/Cancel/Discard dan native close dirty/clean. Setelah view konfirmasi penuh dihapus, `restore_input_focus` memilih query Go to Line/Find yang masih terbuka atau operasi scoped Source; New juga mengembalikan fokus input. Title menyertakan aksi konfirmasi agar status terlihat di window manager. Akselerator Alt+S/Alt+D dikirim ke controller (hanya efektif saat pending/ready), disaring dari fallback Source, dan tidak memakai Ctrl+Alt/AltGr sebagai app command. Tidak ada perubahan serializer/save worker karena penambahan lifecycle ini.

## Batas tanggung jawab

- **`app.rs`**: state UI dan pemilihan mode, routing message, subscriptions, dan layout utama. Tidak memelihara salinan terpisah dari teks/path/dirty/revisi.
- **`app/file_actions.rs`**: koordinasi dialog/worker, pengamanan aksi destruktif, dan penerapan hasil operasi. Berinteraksi dengan model dokumen dan tracker lewat API mereka.
- **`app/editing.rs`**: guard bersama untuk mutasi Source (prompt/Open/Preview), orkestrasi undo/redo, formatting, pencarian, dan replacement. Save/ekspor biasa tidak memblokir edit.
- **`navigation.rs` / `app/navigation.rs` / `ui/navigation.rs`**: parsing koordinat dan state murni / koordinasi guard-caret-fokus / panel Go to Line.
- **`line_edit.rs`**: transaksi raw Source untuk Duplicate/Delete/Move Lines; rentang baris bersama dengan formatter dan indent.
- **`commands.rs`**: enum message dan aksi yang ditunda, serta pemetaan shortcut aplikasi. Binding editing native berada di `editor.rs` agar shortcut tidak menelan input biasa/AltGr.
- **`document/state.rs`**: memiliki satu buffer `text_editor::Content`, path aktif, ID dokumen, revisi operasi monotonik, checkpoint konten, checkpoint terakhir disimpan, dan bounded history. Edit efektif melalui `apply`/`change`; penggantian dokumen melalui `replace`; hasil save melalui `mark_saved`.
- **`document/history.rs`**: dua stack snapshot teks/caret/checkpoint, grouping insert/deletion tervalidasi, batas gabungan langkah dan byte teks. Tidak tahu widget atau path.
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
