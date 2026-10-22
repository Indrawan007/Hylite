# Checklist iterasi Source 4a–4j

**Status: implementasi awal, bukan Tahap 4 lengkap atau aplikasi siap pakai.**

## Hasil yang benar-benar dijalankan

- Harness mandiri memakai modul produksi `history`, `search`, `text_edit`, `block_edit`, `line_edit`, `syntax`, `continuation`, `navigation`, `operations`, dan `document/snapshot`: **150 passed, 0 failed**, Rust 1.88, `-D warnings`.
- Harness tidak mengimpor Iced, tidak membuka dialog, dan tidak menyentuh konfigurasi pengguna.
- Format sumber, pemeriksaan whitespace diff, dan metadata Cargo tanpa dependensi diperiksa terpisah. Ketiganya bukan type-check aplikasi.
- Clippy 1.88 pada harness inti juga lolos dengan `-D warnings`; ini tidak memeriksa modul aplikasi/Iced.
- Build/Clippy/seluruh suite Cargo tetap **belum terverifikasi**, karena arsip dependensi crates.io tidak tersedia di lingkungan ini; pemeriksaan offline terbaru berhenti pada indeks paket `dirs` yang belum tersedia, sementara pengujian sebelumnya terhalang arsip `ab_glyph v0.2.32`. Tidak ada hasil GUI/Arch/Hyprland yang dinyatakan lulus.

Jalankan ulang inti tanpa unduhan crate (cukup Bash dan rustc):

```sh
./scripts/test-source-core.sh
# RUSTC=/path/to/rustc ./scripts/test-source-core.sh --nocapture
# Opsional, setelah rustup component add clippy:
# RUSTC="$(rustup which clippy-driver)" ./scripts/test-source-core.sh
```

Cakupan 150 test tersebut: batas history gabungan, cabang redo, teks/caret/state ID dalam record, rentang Unicode, literal/non-overlap, wrap navigation, invalidasi cache, penggantian kosong/non-rekursif, validasi slice, marker formatting, indent/unindent, dan penolakan token operasi yang salah/kedaluwarsa. Tambahan 4b meliputi normalisasi/toggle heading/list/quote, checkbox, escaping label, fence dinamis, italic pada bold, dan matriks semua rentang valid pada fixture Unicode/multiline. Tambahan 4c meliputi subset token syntax, Unicode/escaping, state fence, replay setelah edit/insert/delete, reset render key/palet, disabled, long-line fallback, budget pada input adversarial, dan matriks span UTF-8. Tambahan 4d mencakup 15 test grouping dengan clock eksplisit: delta UTF-8, kelas teks/whitespace, idle/durasi/scalar cap, checkpoint save, perubahan caret/state, clock mundur, atomic boundary, redo, no-op, serta budget memory. Tambahan 4e mencakup 17 test planner Enter: marker/separator/indent, nomor/zero padding/overflow, task reset, empty item/ASCII blank, midline Unicode, caret invalid, thematic rule, batas baris, serta scanner fence root/quote/list/nested-marker dan false closer. Tes scanner leksikal ini bukan pembuktian guard parser atau integrasi native. Tambahan 4f mencakup 9 test resolver/panel Go to Line (alamat 1-based, Unicode scalar/byte, tab, empty/trailing line, EOL, no-clamping, input malformed/overflow/panjang, matriks semua batas scalar, lifecycle panel) serta 1 test invalidasi target Find tanpa kehilangan hasil cache. Tambahan 4g mencakup 16 test delta/grouping deletion: Backspace/Delete, arah/caret/state, prefix/suffix Unicode, kelas/atomic boundary, waktu/cap, checkpoint, no-op/redo/branch, multi-scalar/grapheme fallback, input offset invalid, matriks UTF-8, serta budget/missing base. Seluruh clock grouping memakai waktu eksplisit, bukan sleep. Tambahan 4h mencakup 15 test pure operasi baris: duplikasi literal/kolom Unicode, seleksi parsial/kolom nol, EOF dengan/tanpa LF dan baris kosong, separator hapusan, rentang invalid, serta matriks semua rentang UTF-8 yang valid dan pembalikan insertion ke source semula. Tambahan 4i mencakup 14 test pindah baris/blok: adjacent swap dua arah, caret Unicode, end-zero selection, blank/EOF/sentinel, no-op identik/boundary/LF-policy, literal markup, inverse round-trip serta permutasi Vec baris independen pada seluruh rentang valid. Tes invariant memeriksa panjang/jumlah LF/status LF terminal, multiset baris, serta prefix/suffix luar transaksi. Tes invalid range lama juga diperluas ke MoveUp/Down. Tambahan 4j mencakup 4 test gabungan: 32.768 langkah workflow dengan oracle history scalar, 8.192 langkah budget invariant, checkpoint captured-save lintas transaksi, dan 192 variasi kandidat delta malformed/no-op. Regresi tersebut memakai API modul core; rig bukan Document/controller/native widget.

## Detail hardening 4j yang benar-benar dijalankan

- Trace model: 64 seed tetap × 512 operasi awal; membandingkan actual record, scalar frame, immediate undo/redo checkpoint dan keberadaan grup pada setiap langkah. Seluruh undo/redo yang tersisa di-drain lagi. Tidak ada sleep atau crate generator; kegagalan trace menyebut seed/step.
- Operasi campuran mencakup primitive insert/delete, atomic replacement, line ops, Enter planner, indent, Find/Replace All, Go to Line dan boundary capture save. Query count dibandingkan pencocokan scalar non-overlap; active range diperiksa UTF-8/source slice.
- Budget: 4 limit entri × 4 limit byte × 512 operasi; memeriksa batas gabungan stack, record caret, peek/restore dan base grup. Ini invariant checking, bukan oracle urutan trim yang independen.
- Tes checkpoint eksplisit memastikan History bisa mencapai record capture sebelum transaksi berikutnya, lalu redo. **Tidak memanggil native mark_saved atau menulis ke disk.** Tes malformed memeriksa kandidat yang ditolak tidak bergabung dengan grup sebelumnya; no-op tidak menambah record.
- Oracle memakai scalar vectors dan stack sendiri untuk history. Transaksi formatter/planner diterapkan dari TextChange produksi yang sama ke kedua representasi, sehingga ini tidak menjadi pembuktian independen kedua tentang semantik formatting. Snapshot/revisi rig adalah stimulus cache, bukan hasil Document native.
- Tidak ada perubahan UX/runtime produksi pada 4j; hanya penambahan test module dan dokumentasi. 150 adalah jumlah fungsi tes yang lulus, **bukan 40.960 tes GUI**. Loop generator menjalankan 40.960 langkah awal plus drain/fixture tambahan.

Untuk fokus pada empat tes itu:

```sh
./scripts/test-source-core.sh workflow_tests --nocapture
```

## Verifikasi wajib pada toolchain lengkap

```sh
cargo fmt --all -- --check
rustfmt --edition 2021 --check tools/source-core-tests.rs
./scripts/test-source-core.sh
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo build --locked
```

Test tambahan yang bergantung pada Iced sudah ditulis di `document/state_tests.rs`, `app/tests.rs`, `editor.rs`, dan `commands.rs`, tetapi **belum dijalankan**. Termasuk checkpoint tersimpan saat undo/redo, undo selama save, state ID cabang baru, no-op mempertahankan redo, trailing LF/Unicode/caret, reset history, formatting seleksi native, Replace All satu transaksi, stale match, prompt/Open/Preview guard, binding lokal, serta kolom Unicode. Regresi tambahan 4b mencakup atomic undo heading/link/fence native, caret UTF-8, no-op/rejection mempertahankan caret/redo, guard prompt/Open/Preview, invalidasi Find, edit block selama save, dan binding formatting lokal. Test 4c untuk adapter Iced, warna/font, konfigurasi boolean, dan lifecycle render key dokumen juga sudah ditulis tetapi belum dijalankan. Test 4d native untuk burst Unicode/revisi, capture save, seleksi, caret Find, paste, save invalid, command/focus barrier, dan capture melalui controller langsung juga belum dijalankan. Test 4e tambahan untuk transaksi/revisi/checkpoint, undo/redo, pemisahan burst, selection/opt-out, parser code range (termasuk false closer kompleks), refresh Find, guard controller, edit selama save, binding Enter dan konfigurasi juga **belum dijalankan**. Test 4f controller/model/shortcut untuk koordinat native, dirty/revisi/undo/redo, selection pada input invalid, resolusi setelah edit, reset dokumen/stale submit, Find, guard prompt/Open, save, Escape/Preview, grouping dan binding Ctrl+G juga **belum dijalankan**. Test 4g native/controller untuk burst Backspace/Delete, Unicode/caret/revisi, selection/direction, newline join, save checkpoint, no-op/redo, navigation barrier, Find serta guard prompt/Open/Preview juga **belum dijalankan**. Test 4h native/controller/binding untuk loader yang menjaga terminal empty line, caret/selection, atomic undo/redo/dirty/revisi/checkpoint, no-op/redo, burst barrier, Find, prompt/Open/Preview, save dan Ctrl+Shift+D/K juga **belum dijalankan**. Test 4i native/controller/binding untuk move Unicode/caret/selection, atomic undo/redo/dirty/checkpoint, EOF/LF/no-op/redo, burst barrier, Find/save, perluasan guard, dan focused bare Alt+arrows juga **belum dijalankan**. Harness 150 test tidak membuktikan kasus-kasus integrasi ini.

## Pengujian desktop manual

Gunakan salinan percobaan, sesi Wayland/Hyprland nyata, dan konfigurasi terpisah:

```sh
scratch=$(mktemp -d)
XDG_CONFIG_HOME="$scratch" cargo run --locked
```

Semua item berikut masih perlu diuji, bukan daftar hasil kelulusan:

### History dan save

- [ ] Ketik satu burst, Paste, Enter, Backspace/Delete, cut; insert dan deletion single-scalar berurutan dapat dikelompokkan dalam grup terpisah, aksi atomic tetap satu per langkah. Navigasi/copy/no-op tidak membuat dirty/history.
- [ ] Simpan, edit, undo menuju checkpoint tersimpan: penanda dirty hilang. Undo menjauhi checkpoint: dirty muncul. Redo kembali: clean lagi.
- [ ] Undo saat save biasa berjalan: hasil save mencatat state yang benar-benar ditulis, bukan otomatis membersihkan buffer saat ini. Path Save As tetap diperbarui hanya sesudah sukses.
- [ ] Setelah undo, ketik di cabang baru: redo hilang dan state ID tidak digunakan ulang. Ketik balik teks yang identik tidak otomatis menjadi checkpoint lama.
- [ ] New/Open menghapus history; hasil save/open lama tidak mengubah dokumen pengganti. Ulangi checklist Tahap 2.
- [ ] Coba batas history di dokumen sintetis. Maksimum gabungan 200 entri / 8 MiB **byte teks record**, bukan total RAM. State melebihi budget mungkin tidak bisa dipulihkan. Periksa status peringatan, jangan memakai data penting.

### Caret, Unicode, dan selection

- [ ] Uji ASCII, `é`, emoji, combining mark, CJK, RTL, soft-wrap, baris kosong, beberapa LF di akhir, dan IME/paste. Tidak boleh merusak teks atau memindahkan caret ke byte UTF-8 yang tidak valid.
- [ ] Undo/redo memulihkan teks dan caret, **bukan anchor seleksi**. Periksa caret setelah format, indent, dan navigasi Find.
- [ ] Kolom status/Tab memakai Unicode scalar, bukan byte, grapheme, atau lebar visual; word count memakai pemisahan whitespace.
- [ ] Uji scrolling dan fokus setelah rekonstruksi buffer: API Iced 0.13 tidak memiliki setter caret/selection publik. Perilaku viewport native belum dibuktikan.

### Find/Replace

- [ ] Ctrl+F / Ctrl+H memfokuskan query. Mengetik query/replacement tidak mengedit dokumen atau history dokumen.
- [ ] Pencarian literal, case-sensitive, non-overlap; query kosong menghasilkan nol match. Tidak ada regex, case-fold, atau whole-word.
- [ ] Enter/F3 maju, Shift+F3 mundur; di ujung berputar. Cek jumlah, posisi caret, dan konteks `⟦match⟧` pada panel; **tidak ada highlight/seleksi native**.
- [ ] Next dulu lalu Replace satu hasil, termasuk replacement kosong. Replace All tidak mencari ulang isi penggantinya, dan dapat di-undo satu langkah.
- [ ] Edit/undo/query berubah menginvalidasi match aktif. New/Open tidak boleh memakai rentang lama. Replace satu hasil tanpa match aktif tidak mengubah dokumen.
- [ ] Find dari Preview berpindah ke Source. Beralih ke Preview menutup panel. Escape menutup panel sebelum keluar Zen; prompt/error tetap lebih prioritas.

### Formatting, indentation, dan guard

- [ ] Ctrl+B tetap sidebar; Ctrl+Shift+B bold, Ctrl+I italic, Ctrl+E inline code. Ctrl+Shift+E tetap ekspor, tidak ikut memformat.
- [ ] Dengan seleksi, marker membungkus source. Tanpa seleksi, sisipkan placeholder `text`; caret sebelum penutup. Pilih seluruh source bertanda untuk toggle marker.
- [ ] Inline code multiline/backtick internal yang tidak didukung ditolak, bukan menghasilkan markup rusak. Ini formatter source sederhana, bukan AST-aware.
- [ ] Tab tanpa seleksi menuju tab stop; dengan seleksi mengindentasi seluruh baris tersentuh. Seleksi berakhir pada kolom nol baris berikutnya tidak mengindentasi baris itu. Shift+Tab menghapus satu tab atau hingga `tab_size` spasi per baris.
- [ ] Toolbar dan shortcut mengikuti guard yang sama: prompt/Open/Preview menolak mutasi; save/ekspor biasa mengizinkan edit. Ctrl+Z saat fokus query bukan undo dokumen.
- [ ] Menu Blocks… dan Ctrl+1–6 mengubah/toggle heading; Ctrl+Shift+L/O/T mengubah bullet/numbered/task list. Cek seleksi campuran, baris kosong, indent, `1)`/`+`/`*`, dan baris akhir kolom nol. Quote men-toggle satu level.
- [ ] Cek checkbox `[x]`/`[X]`/`[ ]` dalam seleksi campuran; checked state dipertahankan ketika menormalisasi task. Toggle lepas atau konversi ke list biasa menghapus checkbox. Numbered list baru dihitung mulai 1, melewati baris blank.
- [ ] Ctrl+K: bracket/backslash label di-escape; multiline/control ditolak. Lengkapi URL setelah `https://` sebelum `)`. Tanpa seleksi label `text`. Tidak ada popup validasi URL/toggle link.
- [ ] Ctrl+Shift+C: code fence membungkus baris penuh; backtick 3/4/lebih panjang di payload tidak menutup wrapper. Cek caret tetap di body, newline akhir dipertahankan, dan undo/redo satu langkah.
- [ ] Formatting no-op/rejection tidak membuat history atau membuang redo, dan tidak melompat ke ujung seleksi yang berbeda. Seleksi memang belum dipulihkan.
- [ ] Coba formatter di dalam fenced/indented code, nested list, dan quote: formatter tidak membaca AST di luar rentang. Periksa efek kontekstual; ini belum formatter struktural ala MarkText.
- [ ] Ulangi Source, Split, Zen, jendela sempit, font alternatif, dan config `tab_size` 1/16. Tombol keyboard tetap dapat dipakai saat toolbar tersembunyi di Zen.

### Syntax highlighting (4c)

Seluruh item berikut **belum diverifikasi di GUI**:

- [ ] Source/Split/Zen memberi warna subset token yang sama, tanpa mengubah isi file, dirty, caret, seleksi, maupun langkah undo saat hanya render/scroll.
- [ ] Heading ATX 1–6, bullet/numbered/task, quote, thematic rule, strong/emphasis, inline code, inline link/image. Literal escape dan intraword underscore tidak menjadi emphasis.
- [ ] Buka/tutup backtick/tilde fence. Isi block tidak menjadi heading/list/emphasis. Fence penutup harus sama jenis, minimal sepanjang pembuka, dan tanpa trailing non-whitespace. Bahasa code tidak diberi syntax sendiri.
- [ ] Edit/hapus/insert pembuka/penutup fence lalu scroll jauh ke bawah/atas: warna mengikuti konteks terbaru, tidak state lama. Ulangi undo/redo, Replace All, format block, Find/F3, New/Open, termasuk dokumen berbeda dengan jumlah baris sama.
- [ ] Tema terang/gelap dan font custom: warna terbaca, font/ukuran/weight tidak diganti. Cek IME, RTL, combining mark, emoji, serta soft-wrap dan posisi caret.
- [ ] Restart dengan `editor.syntax_highlighting = false`: Source plain, preview tetap renderer terpisah. Tidak ada live reload; field bukan boolean menolak konfigurasi dengan warning tanpa overwrite.
- [ ] Baris >16 KiB tampil plain, bukan dipotong. Fence pada baris panjang tetap mengubah state baris berikutnya. Input banyak delimiter yang tidak ditutup dapat menyisakan suffix plain ketika budget lookahead habis.
- [ ] Coba Setext, reference link, raw HTML, tabel, nested containers/fences, emphasis kompleks: ini subset leksikal; jangan menganggap warna sebagai validasi Markdown.

### Typing coalescing (4d)

Seluruh skenario desktop di bawah masih **belum dijalankan**:

- [ ] Ketik `hello`, Ctrl+Z sekali menghapus burst itu; redo mengembalikan teks/caret. Coba Unicode, combining mark, emoji/ZWJ; ini scalar/UTF-8-aware, bukan grapheme segmentation lengkap.
- [ ] Teks, whitespace, dan ASCII punctuation memiliki batas berbeda; underscore masih teks. Ketikan berhenti ≥750 ms, burst berjalan ≥5 s, dan karakter ke-65 membuat langkah baru. Waktu berdasarkan pemrosesan aksi.
- [ ] Pindahkan caret (bahkan kembali ke posisi sama), pilih teks, copy, Tab, format, Find/F3, command UI, dan pindah fokus jendela: ketikan berikutnya tidak bergabung dengan grup sebelumnya.
- [ ] Paste dan IME commit sebagai Paste tetap atomic. Backspace/Delete/Enter/cut serta replacement seleksi tidak dicoalesce dengan insert di sekitarnya.
- [ ] Ketik `ab`, save, ketik `cd` selama save; undo kembali ke `ab` dan menjadi clean sesudah save berhasil. Undo menjauhi checkpoint kembali dirty. Save gagal/cancel tidak memalsukan checkpoint.
- [ ] Simpan setelah burst, undo sebelum acknowledgment, lalu terima acknowledgment: dirty harus mengikuti checkpoint yang ditulis, bukan buffer saat ini. Ulangi Save As/path dan prompt Save & Continue dari Tahap 2.
- [ ] Edit baru setelah undo menghapus cabang redo. No-op mempertahankan redo. Render/highlighting/scroll redraw tanpa command tidak membuat revisi atau undo step tambahan.
- [ ] Warning budget menyebut 200 langkah/8 MiB, bukan 200 karakter. Uji pada dokumen sintetis; grouping tidak menghapus kebutuhan salinan buffer O(n) per aksi dan bukan proteksi crash/recovery.

### Continuation list (4e)

Seluruh skenario desktop ini masih **belum dijalankan**:

- [ ] Enter di akhir bullet `-`/`+`/`*`, nomor `009)`/`9.`, task `[x]`/`[X]`/`[ ]`: prefix/separator/0–3 spasi awal dipertahankan, nomor bertambah, checkbox baru kosong.
- [ ] Enter di tengah `- café世界`: suffix tidak hilang, caret setelah prefix baru, undo/redo satu langkah; burst ketikan sebelum/sesudahnya tetap terpisah.
- [ ] Item kosong dengan spasi/tab melepas marker/checkbox di tempat, mempertahankan indent, tanpa LF tambahan. NBSP tidak dihapus sebagai blank. Uji item sebelum baris berikutnya dan di EOF.
- [ ] Shift+Enter dan config `auto_continue_lists = false` (restart) tetap Enter native. Config lama tanpa field default true; nilai bukan boolean ditolak tanpa overwrite.
- [ ] Seleksi aktif, caret di prefix, quoted list, ≥4 spasi/tab, nomor overflow, thematic rule dan baris panjang tetap native tanpa pemotongan data. Ini bukan auto-indent/outdent atau dukungan nested list struktural.
- [ ] Fence root/list/quote, nested container, delimiter payload yang tampak seperti closer, dan EOF fence tak tertutup tidak menambah marker. Ulangi dengan syntax highlighting disabled. Scanner ambigu boleh fallback lebih lama, bukan menebak struktur.
- [ ] Prompt/Preview/Open menolak command; Save/ekspor biasa tetap mengizinkan edit. Find di-refresh dan active match diinvalidasi setelah Enter efektif. Undo ke checkpoint yang disimpan memulihkan clean.
- [ ] Ulangi Source/Split/Zen, soft-wrap, font alternatif, emoji/combining/RTL dan IME. Periksa fokus/viewport setelah rekonstruksi. Uji latensi pada dokumen sintetis; parser kandidat belum incremental.

### Go to Line (4f)

Seluruh skenario desktop ini masih **belum dijalankan**:

- [ ] Ctrl+G dan tombol Go to Line mengisi posisi saat ini dan memfokuskan input. Enter/Go menuju `12` atau `12:5`, menutup panel dan meminta fokus Source; Close/Escape tidak mengubah teks/caret. Ulangi Source/Split/Zen dan Preview → Source.
- [ ] Kolom sesuai status bar: `é`, CJK, RTL, tab, combining mark, emoji/ZWJ; endpoint EOL, dokumen kosong, blank line dan LF terakhir dapat dituju. Bukan kolom byte/grapheme/display width atau baris soft-wrap.
- [ ] Nol, negatif, `+1`, digit Unicode, `1:`, `1:2:3`, whitespace internal, overflow, >64 byte dan lokasi di luar rentang memberi error tanpa clamping, kehilangan seleksi, atau perubahan caret. Mengedit query menghapus error.
- [ ] Edit buffer setelah panel terbuka: Go memakai rentang terbaru. New/Open sukses menghapus state panel; submit setelah panel tertutup tidak boleh meloncat di dokumen baru. Prompt/Open memblokir query/submit; save/ekspor biasa tidak.
- [ ] Navigasi tidak mengubah dirty, revisi/state ID, checkpoint tersimpan, stack undo atau redo; ketikan sebelum/sesudah command menjadi burst berbeda. Save yang sedang berjalan tetap mencatat checkpoint yang sama sesudah navigasi.
- [ ] Find dan Go to Line bergantian, termasuk F3. Navigasi berhasil membersihkan active match sehingga Replace memerlukan Next lagi, tetapi query/replacement/count Find dipertahankan. Preview menutup panel. Escape tetap memprioritaskan prompt/error sebelum panel, lalu Zen.
- [ ] Caret benar-benar terlihat dan fokus kembali ke editor setelah submit/close, termasuk soft-wrap, scroll jauh, font alternatif, window sempit, IME dan RTL. API rekonstruksi caret/fokus ini belum dibuktikan melalui GUI; jangan menganggap test resolver sebagai jaminan viewport.
- [ ] Uji latensi dokumen sintetis. Belum ada indeks baris incremental atau gutter; scan/salin/rebuild masih O(n). Nomor baris terpisah tanpa data layout/scroll Iced tidak boleh dianggap selaras.

### Deletion coalescing (4g)

Seluruh skenario desktop ini masih **belum dijalankan**:

- [ ] Backspace beberapa karakter teks sederhana, Ctrl+Z sekali memulihkan burst berikut caret asal; redo menghapusnya kembali. Ulangi Delete dari tengah baris dan Unicode single-scalar tanpa merusak prefix/suffix.
- [ ] Ubah arah Backspace ↔ Delete, insert ↔ delete, dan teks ↔ whitespace: tiap jenis/kelas menjadi grup berbeda. ASCII punctuation (kecuali `_`), tab, newline join dan control tetap atomic.
- [ ] Penghapusan seleksi satu scalar maupun banyak scalar tetap atomic. Uji combining mark, emoji/ZWJ, RTL, IME dan penghapusan native multi-scalar: jika delta tidak persis satu scalar/caret lokal, jangan digabung. Algoritme hapus native tidak diganti, bukan grapheme segmentation penuh.
- [ ] Jeda ≥750 ms, total burst ≥5 s dan karakter ke-65 membuka langkah baru. Navigasi, selection, Find/Go to Line, save, toolbar command, undo/redo dan window blur menutup grup, meskipun caret tidak berubah.
- [ ] Delete di EOF / Backspace di BOF tanpa perubahan tidak menambah dirty/revisi/history atau membuang redo, tetapi menutup grup. Setiap delete efektif tetap mempunyai revisi/state ID baru.
- [ ] Hapus sebagian teks, save, lanjut hapus selama worker berjalan: undo kembali ke checkpoint yang benar-benar ditulis dan menjadi clean setelah acknowledgment. Undo menjauh/redo kembali mengikuti checkpoint; path Save As tetap benar.
- [ ] Find count/cache di-refresh dan active match diinvalidasi setelah edit efektif, termasuk deletion yang tergabung dalam undo. Prompt/Open/Preview memblokir edit; save/ekspor biasa mengizinkan.
- [ ] Ulangi Source/Split/Zen, soft-wrap, native caret/viewport, dan dokumen sintetis mendekati budget history. Tidak ada jaminan recovery/latensi atau batas RAM total dari grouping dan budget 200 langkah/8 MiB.

### Operasi baris (4h)

Seluruh skenario desktop ini masih **belum dijalankan**:

- [ ] Ctrl+Shift+D/K hanya di Source focused, tidak di query Find/Go to Line atau AltGr/logo. Ctrl+K tetap link. Tombol Duplicate/Delete Lines memakai guard sama dengan shortcut. Ulangi Source/Split/Zen (toolbar tersembunyi di Zen).
- [ ] Duplicate tanpa seleksi menyalin satu baris ke bawah dan mempertahankan kolom caret UTF-8. Seleksi parsial beberapa baris menyalin seluruh blok dan caret berada di awal blok salinan; endpoint kolom nol tidak memasukkan baris berikutnya. Anchor selection memang tidak dipertahankan.
- [ ] Indentasi/tab, `009)`, `[X]`, Unicode, combining/RTL dan markup disalin literal, termasuk di code fence; tidak ada renumber/reset task atau AST balancing.
- [ ] Uji baris pertama/tengah/akhir, EOF dengan/tanpa LF, beberapa LF terakhir, empty document dan Select All. Duplikasi nonempty EOF tidak membuat LF terminal baru; duplikasi blank line menambah satu LF. Delete EOF mengonsumsi LF sebelum blok; delete nonfinal mengonsumsi LF sesudahnya; clipboard tetap sama.
- [ ] New/Open/load/replace dokumen dengan `a\n`, `é✨\n\n` dan baris blank-only: baris kosong terakhir tidak hilang, caret awal 0, clean/revisi 0, tanpa undo record pemuatan. Ini regresi initializer native, bukan dibuktikan oleh harness core.
- [ ] Satu aksi efektif satu undo step dengan fresh revision/state ID; undo/redo memulihkan teks/caret/dirty checkpoint. Ketikan/hapusan di sekitarnya terpisah. Delete empty document tidak menambah history/revisi/dirty atau menghapus redo.
- [ ] Find count diperbarui dan active match diinvalidasi. Save/ekspor biasa mengizinkan operasi; undo ke snapshot save menjadi clean setelah acknowledgment. Prompt/Open/Preview menolak operasi tanpa mutasi buffer/history/caret.
- [ ] Periksa fokus, scroll, soft-wrap, IME/RTL dan native selection sesudah rekonstruksi. Uji dokumen sintetis mendekati budget history; snapshot/copy/rebuild masih O(n), bukan jaminan recovery/performa file besar.

### Pindahkan baris/blok (4i)

Seluruh skenario desktop ini masih **belum dijalankan**:

- [ ] Alt+↑ / Alt+↓ pada Source focused dan tombol Move Up/Down menukar satu baris/blok dengan satu adjacent line; urutan di dalam blok tetap sama. Ctrl+Alt/AltGr, Shift+Alt, logo, Left/Right dan input Find/Go to Line tidak memindahkan source. Periksa konflik binding compositor, bukan menganggap shortcut selalu diteruskan ke aplikasi.
- [ ] Endpoint selection di kolom nol tidak memasukkan baris berikutnya. Caret collapsed mengikuti byte-column UTF-8; selected block mengakhiri selection dan menaruh caret di awal blok. Shortcut berikutnya bekerja pada baris caret kecuali blok dipilih ulang. Undo tidak memulihkan anchor.
- [ ] Indent/tab, Unicode/combining/ZWJ/RTL, markup/list/task/quote/fence tetap literal. Bukan pemindahan AST/container; struktur Markdown bisa berubah sesuai baris yang dipindah.
- [ ] Uji LF akhir ada/tidak ada, beberapa blank line, EOF dan Select All. Final empty line sesudah LF tidak menjadi target. Blank biasa bisa dipindah, tetapi gerakan blank ke EOF tanpa LF yang mengubah status LF terminal ditolak. Tidak boleh ada baris hilang/duplikat atau LF ekstra.
- [ ] Batas atas/bawah, swap identik dan swap yang melanggar kebijakan LF adalah no-op: snapshot/dirty/caret/redo tidak berubah; tidak ada entry undo baru. Anchor seleksi tetap dapat dikolapskan seperti formatter lain.
- [ ] Move efektif atomic dengan fresh revision/state ID. Insert/delete bursts di kedua sisi tetap terpisah; undo/redo dan checkpoint Save As/during-save benar. Find refresh menginvalidasi active match meskipun count tidak berubah.
- [ ] Prompt/Open/Preview memblokir operasi; save/ekspor biasa mengizinkan. Ulangi Source/Split/Zen, toolbar tersembunyi, scrolling jauh, soft-wrap, IME/RTL, serta fokus/viewport sesudah rekonstruksi. API native dan performa file besar belum dibuktikan oleh harness core.

## Di luar iterasi ini

Syntax highlighting CommonMark lengkap/embedded language, gutter nomor baris, formatter AST/container-aware, editor URL, continuation/outdent list struktural, pengelompokan deletion multi-scalar/grapheme segmentation penuh, native match highlighting, pemulihan seleksi, dan performa file besar belum diselesaikan. Tidak ada recovery/autosave/conflict detection. Snapshot penuh dan rekonstruksi buffer masih menyalin teks O(n); jangan menganggap batas history sebagai jaminan RAM/latensi aplikasi.
