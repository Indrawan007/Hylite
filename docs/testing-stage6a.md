# Stage 6a — prototipe toggle task dari preview

Interaksi edit preview pertama: **hanya checkbox task list kanonis**. Ini belum
WYSIWYG umum, editable AST, cell editing, glyph/selection mapping atau scroll
sync. Source tetap satu-satunya buffer otoritatif; tidak ada serialisasi AST.

## Perilaku dan batas

- Task GFM yang diparse dari list item menampilkan checkbox native. Klik pada
  Preview atau Split men-toggle tepat satu byte isi marker `[ ]` / `[x]` / `[X]`.
  Saat mencentang dipakai `x` lowercase; saat uncheck dipakai satu spasi. Marker
  lain, prefix/nomor, Unicode, whitespace, body dan code literal tidak diubah.
- Mode tidak berubah. Native Source caret pindah ke setelah karakter yang
  ditoggle (sebelum `]`); selection dikolapskan. Split memulihkan konteks fokus
  Find/Go to Line yang masih terbuka, selain itu Source. Preview tidak meminta
  fokus Source yang tidak ada.
- Checkbox berubah dari proyeksi Source terbaru, bukan state widget optimistis.
  Satu effective toggle ialah satu transaksi Atomic melalui `Document::change`:
  revisi/state ID/dirty diperbarui, redo branch lama dibuang dan cache preview
  invalidated. Undo mengembalikan byte asli, termasuk kapital `X` dan caret lama;
  budget history 200 entri/8 MiB tetap berlaku, bukan jaminan undo file besar.
- **Undo/Redo masih memakai kebijakan Source/Split sebelumnya.** Dari Preview
  pilih Split atau Source untuk memakai Undo; tidak menambahkan shortcut edit
  umum pada Preview. Klik checkbox bukan izin bagi native typing/formatting lain.
- Task-like text dalam code/HTML/table/plain escaped text bukan checkbox edit.
  Marker nonkanonis yang parser terima (misalnya tab di dalam `[\t]`) tetap task
  terlihat tetapi checkbox disabled; tidak dinormalisasi diam-diam.
- Tombol navigasi blok dan klik kanan sel 5d/5e tetap tersedia. URL/image/HTML
  tidak mendapat aksi baru; blok selain task checkbox tetap tidak editable.

## Provenance dan guards

Model `TaskMarker` menyimpan checked state dan optional canonical marker range.
Range event parser dapat memuat sampai 3 spasi ASCII sebelum token. Resolver
menerima hanya suffix `[ ]`/`[x]`/`[X]` yang cocok dengan checked state; prefix
harus 0–3 spasi, bukan pencarian marker bebas. Token unsupported menghasilkan
range None tetapi state task tetap ditampilkan. Tidak ada offset glyph.

`preview_task::Target` membawa full Snapshot, exact item range, marker range dan
expected checked state. Planner core memeriksa snapshot, UTF-8/bounds, token dan
containment; planner saja tidak membuktikan konteks Markdown. Document juga
menuntut cache warm dan exact item/marker/state membership pada model saat ini.
Target cold, forged, code marker, stale/double-click message, edit→undo source
identik atau dokumen pengganti tidak di-rebase atau di-parse demi message.

Controller hanya menerima dari mode Preview/Split, memblokir prompt/Open dan
mengizinkan Save/Export biasa tanpa mengonsumsi token. Ack save checkpoint lama
tidak membersihkan perubahan task berikutnya. Setelah sukses Find di-refresh
terhadap source/revisi baru; peringatan history limit tetap dipakai. Invalid
message tidak mengganti teks, caret, selection atau render key; hanya status
controller dapat berubah. UI command tetap merupakan typing-group boundary.

## Regresi yang ditambahkan

- **10 core**: one-byte diff, uppercase/lowercase, round trip, bounded prefix,
  noncanonical/state mismatch, UTF-8/overflow/empty bounds, full snapshot, item
  containment, event-vs-normalized range dan kontrak membership terpisah.
- **3 model**: exact ranges dalam tight/loose/nested/quoted/ordered task, disabled
  tab marker, task-like code/HTML/table/escape. Validator model lama juga menuntut
  canonical token berada di dalam item range.
- **8 Document**: Atomic undo/redo + invalidasi, nested source, forged target
  termasuk parent item yang terlalu luas, selection/cache/no-op retention,
  stale/double/undo-same-text, cold/replace, code/disabled, redo branch dan save.
- **2 adapter**: stamped target marker/item/state; unsupported/non-task disabled.
- **5 controller**: hanya explicit task edit bypass Preview guard, stale duplicate,
  Source/prompt/Open/Save/Export guards, Split query refresh/context, save ack lama.
- GUI baru **8 clipboard checks**: fixture Unicode/trailing tab/capital X/code;
  klik checkbox Preview, pindah Split, exact checked Source; satu Undo/Redo;
  klik checkbox Split, Source focus tanpa refocus; Undo masing-masing task edit,
  lalu Undo fixture kembali welcome clean. Semua Copy `refocus=False` dan klik
  hanya pada checkbox produk, bukan editor Source. Expected bytes literal.
- Satu helper Python mengunci urutan expected source, dua klik checkbox, whitespace
  serta tidak adanya klik/windowfocus Source. Screenshot `task-preview.png` dan
  `task-split.png` merupakan runtime evidence, bukan pixel assertion.

## Status eksekusi

Lokal **172 core strict** dan **20 Python** lulus; format/diff diperiksa. Gate
Cargo/Clippy/build/Iced serta klik GUI native baru masih menunggu CI; tes core
bukan pengganti runtime native.

## Checklist manual — belum dinyatakan lulus

- [ ] Checkbox nested/loose/quoted, multiline, disabled, resize dan Light/Dark.
- [ ] Source caret/viewport jauh serta query focus runtime setelah toggle Split.
- [ ] Wayland/Arch/Hyprland, GPU, touchpad, IME/RTL dan portal file.
- [ ] UX Undo langsung Preview sebelum memperluas editing umum.

Parser/render tetap eager O(n) pada cache miss; buffer/history/caret reconstruction
masih O(n), tanpa benchmark speedup atau budget memori total. Tidak menambah
dependensi, mengubah lockfile, worker file, serializer, test hook atau timeout.
Stage 6 baru prototipe sempit ini, bukan pernyataan seluruh tahap selesai.
