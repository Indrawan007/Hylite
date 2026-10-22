# Stage 5e — navigasi provenance sel tabel ke Source

Melanjutkan [Stage 5d](testing-stage5d.md). Ini **bukan edit tabel langsung,
cell selection, glyph hit testing, WYSIWYG atau synchronized scrolling**.

## Kontrak

- Caption tabel menjelaskan **Right-click cell → Source**. Klik kanan pada area
  konten/padding sel dengan rentang sumber non-empty membuka Split (atau tetap
  Split) dan memfokuskan caret pada awal range source sel.
- Klik kiri tetap tidak menavigasi atau mengedit Source. URL tidak dibuka,
  gambar tidak dimuat, dan HTML/math tetap inert. Tombol whole-block 5d tetap
  tersedia; sel pada fallback tabel di atas limit tidak menjadi target.
- Sel padding GFM memiliki range kosong dan tidak diberi action. Sel eksplisit
  berisi whitespace bisa mempunyai range non-empty meskipun run kosong: yang
  dituju ialah provenance raw, bukan glyph/spacer tampilan. Empty range eksplisit
  juga tidak diberi action. Tidak ada tebakan posisi atau clamp ke sel tetangga.
- Awal range dapat berada sebelum spasi atau marker `**`, bukan pada teks hasil
  styling. Quote/list prefix, entity, escaped pipe dan Unicode tidak dianggap
  pemetaan 1:1 display↔source.

## Target dan validasi

`Target` kini mempunyai `Origin::Block` atau `Origin::TableCell`, selain full
Snapshot dan range. Origin sel memuat range parent tabel serta indeks row dan
column **zero-based** (row 0 = header). Factory `block` mempertahankan jalur 5d;
factory `cell` hanya membentuk data, bukan bukti validitas model.

Resolver core memeriksa snapshot dan UTF-8 range, termasuk bahwa range sel
berada di dalam parent range yang valid. Document menuntut cache warm dan exact
membership: node harus tabel dengan parent range yang sama, indeks row/column
harus ada, dan source range sel harus identik. Range non-empty yang sembarang,
indeks salah/overflow, parent tabel lain, role salah dan stale snapshot ditolak.
Guard snapshot awal menolak stale message sebelum membuat raw text snapshot.
Tidak ada reparse untuk message cold, rebase atau pemilihan sel dengan teks sama.

Setelah validasi, controller memakai jalur caret/mode/panel/fokus 5d yang sama:
konfirmasi/Open serta mode Source menolak; Save/Export biasa mengizinkan tanpa
mengonsumsi token. Find/Go to Line ditutup dan current Find dibersihkan setelah
sukses. Navigasi tidak mengubah teks, revisi/checkpoint, dirty atau stack
undo/redo; selection dikolapskan dan typing group ditutup. Cache model tetap
valid meski rekonstruksi caret mengganti native render key.

Renderer membungkus sel actionable dengan `mouse_area.on_right_press`; tidak
menambah baris tombol per sel atau mengubah lebar kolom, alignment, wrapping,
Stack divider overlay, maupun batas 12 kolom/128 row/512 sel. Caption satu baris
bisa wrap pada viewport sempit. Area klik mengikuti box konten/padding sel;
sel dengan konten lebih pendek belum dipaksa setinggi seluruh row.

## Regresi

15 tes Rust baru ditambahkan untuk suite native:

- 4 core: parent bounds/UTF-8, raw marker vs display, empty/stale, serta kontrak
  bahwa bounds resolver saja tidak mengautentikasi indeks model.
- 6 Document: header/body/nested table dan Unicode byte-exact/cache; parent/
  indices/range/role forgery; cold/padded; edit/undo/redo/replace stale; metadata/
  caret/redo retention; fallback tabel tidak menerima origin sel.
- 3 adapter: snapshot/table/row/column tepat, marker strong bukan run glyph,
  disabled padding vs whitespace provenance, URL/image tetap source target.
- 2 controller: Preview→Split pada marker raw tanpa mengubah snapshot/cache/
  redo/query; guard Source/prompt/Open/Save/Export tetap sama.

Probe GUI menambah 4 clipboard checks: fixture Unicode/terminal tab, klik kiri
sel + ketikan `LEAK` tidak boleh menavigasi, klik kanan sel Count lalu `X` harus
mengubah `|café🙂|**3**|` menjadi `|café🙂|X**3**|`, Undo mengembalikan fixture,
dan satu Undo lagi kembali ke welcome clean. Semua Copy `refocus=False`, tidak
ada klik tambahan ke Source. Expected bytes dibuat dari literal fixture, bukan
parser/planner Rust. Satu helper Python mengunci urutan klik kiri/kanan, sentinel,
fixture/expected bytes dan tanpa workaround fokus. Screenshot `cell-preview.png`
dan `cell-split.png` bukan pixel oracle.

## Status eksekusi

Lokal: **162 core** strict mandiri dan **19 helper Python** lulus; format/diff
juga diperiksa. Build/Clippy/Cargo/Iced/GUI native baru masih menunggu CI; hasil
core tidak menggantikan verifikasi tersebut.

Percobaan [CI 39731115281](https://github.com/Indrawan007/Hylite/actions/runs/39731115281)
lulus format/162 core/19 Python, tetapi kompilasi target tes saat lint menemukan
helper controller mengakses modul parser privat (`E0603`). Helper diperbaiki
untuk membuat range dari literal fixture independen; visibility model tidak
diperluas dan tidak ada gate yang dilemahkan. Gate build/Cargo/GUI pada percobaan
tersebut belum dijalankan dan bukan hasil lulus.

## Batas dan pengujian manual yang belum dinyatakan lulus

- [ ] Visibilitas caret dan Source viewport untuk sel jauh/scroll horizontal.
- [ ] Klik nested table, whitespace/empty cell dan variasi wrap/row height di GUI.
- [ ] Caption/kepadatan layout pada Light/Dark, resize/HiDPI/touchpad.
- [ ] Wayland/Arch/Hyprland, GPU, IME/RTL serta portal file.

Source tetap otoritatif. Pencarian node dan rekonstruksi buffer masih O(n),
belum indeks posisi incremental atau klaim peningkatan performa. Cache/budget
history dan fallback parser sebelumnya tidak diperluas. Tidak ada dependency,
lockfile, serializer, file worker, hook tes produksi atau timeout baru.
