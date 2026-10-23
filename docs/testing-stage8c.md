# Stage 8c — gambar read-only di Preview, dengan gerbang path dan ukuran

Sebelum stage ini setiap `![alt](src)` menjadi teks literal
`[image not loaded: …]`. Sekarang gambar **lokal** bisa tampil, dan setiap jalur
yang tidak diizinkan tetap menjadi penanda inert — bukan tebakan, bukan fetch.

Yang dikerjakan:

- `![alt](pic.png)` pada paragraf yang **hanya** berisi gambar itu dirender
  sebagai gambar, dengan batas tampilan 560×420 dan tooltip `alt · W×H`.
- Path relatif di-resolve terhadap **folder dokumen**, bukan working directory
  proses. Dokumen yang belum pernah disimpan (`Untitled`) menolak path relatif.
- URL jaringan **tidak pernah** dibuka: `http:`, `https:`, `file:`, `data:`,
  `//host`, dan skema apa pun ditolak sebelum satu byte pun dibaca.
- Format yang diterima: **PNG, GIF, BMP, JPEG** — hanya itu yang dimensinya bisa
  dibaca dari header tanpa decoder.

## Gerbang (semuanya konstanta, semuanya diuji)

| Gerbang | Nilai | Cara diperiksa |
|---|---|---|
| Ukuran file | `MAX_FILE_BYTES = 4 MiB` | `metadata().len()` sebelum file dibuka |
| Sisi terpanjang | `MAX_DIMENSION = 8 192 px` | header PNG/GIF/BMP/JPEG |
| Total piksel | `MAX_PIXELS = 40 000 000` | `width × height` dari header |
| Header dibaca | `HEADER_BYTES = 64 KiB` | tidak pernah membaca seluruh file untuk memeriksa |
| Path absolut | boleh | dipakai apa adanya |
| Path relatif | butuh folder dokumen | `MissingBase` bila dokumen belum disimpan |
| `..` | boleh | dinyatakan; tidak ada sandbox root di stage ini |
| Symlink | diikuti oleh `fs::metadata` | dinyatakan; gerbang ukuran tetap berlaku |

Semua penolakan menjadi marker yang stabil dan bisa diuji:

| Kondisi | Marker |
|---|---|
| skema/URL | `[image not loaded: remote URL: <src>]` |
| kosong | `[image not loaded: empty destination: <src>]` |
| `%` rusak, NUL, `#frag` | `[image not loaded: invalid path: <src>]` |
| relatif tanpa path dokumen | `[image not loaded: save the document first: <src>]` |
| tidak ada | `[image not loaded: file not found: <src>]` |
| bukan file biasa | `[image not loaded: not a file: <src>]` |
| tidak bisa dibaca | `[image not loaded: cannot read file: <src>]` |
| > 4 MiB | `[image not loaded: file too large: <src>]` |
| > 8 192 px atau > 40 MP | `[image not loaded: image too large: <src>]` |
| selain PNG/GIF/BMP/JPEG | `[image not loaded: unsupported format: <src>]` |

Percent-encoding **didukung** (`my%20pic.png` → `my pic.png`), karena Markdown
nyata sering menulis spasi begitu; escape yang rusak ditolak, bukan ditebak.

## Batas cakupan (disengaja)

- Hanya paragraf yang isinya **tepat satu** gambar yang menjadi gambar. Gambar
  inline di tengah teks, di dalam tabel, atau di dalam sel tetap berupa penanda
  inert — `rich_text` tidak bisa menyisipkan widget. Ini batas yang dinyatakan,
  bukan bug yang tersembunyi.
- Dekode tetap milik `iced` (fitur `image` sudah aktif); kita hanya memutuskan
  **apakah** file boleh masuk ke pipeline itu. Tidak ada decoder sendiri, tidak
  ada crate baru.
- Belum ada: resize/drag gambar, caption, SVG, WebP, kontrol animasi GIF, paste
  gambar ke disk, atau pemuatan dari jaringan.

## Tes

**Rust (988 Cargo, +13 dari 975; 314 strict core, +8 dari 306)**

- `src/images.rs` (8, masuk strict core karena murni `std`): semua skema ditolak
  sebagai `Remote`; path relatif butuh base dan absolut tidak; kosong/`#frag`/
  `%` rusak/NUL ditolak sementara `%20` diterima; dimensi dibaca dari header
  PNG/GIF/BMP (termasuk tinggi negatif BMP)/JPEG dan header rusak → `None`;
  `inspect` menolak file hilang, folder, header terpotong, file > 4 MiB, dan
  header 9000×200 maupun 8000×8000 (tanpa mendekode apa pun); `load`
  menggabungkan resolve + gate; semua label marker stabil.
- `src/document/preview_model.rs` (4): run gambar membawa `src`+`alt` persis
  sementara teks marker lama **tidak berubah** (oracle lama tetap byte-identik);
  gambar campur teks tetap satu run literal; alt kosong tetap kosong; gambar di
  dalam quote/list tetap membawa spec masing-masing dan rentangnya valid.
- `src/document/preview.rs` (1): membangun widget untuk gambar lokal, hilang,
  remote, escape `..`, inline, dan alt kosong — dengan maupun tanpa folder
  dokumen — tidak pernah panic.

**Python (79, +4)**: probe memastikan suite keyboard-only (tanpa klik), urutan
copy sama dengan byte file, jumlah key persis (`ctrl+shift+m` ×6, `ctrl+v` ×1),
dua screenshot, satu exit native; fixture menulis PNG yang benar-benar bisa
didekode, file > 4 MiB, file hilang yang tidak ada, dan URL remote; dispatch
eksklusif; suite menolak jalan tanpa path dokumen.

**GUI (26 suite / 488 pemeriksaan, +5 dari 483)** — suite ke-26 `images`, boot
dengan `hylite doc.md`:

1. argumen launch membuka dokumen, title `doc.md — Hylite`;
2. `Ctrl+Shift+M` → Preview (screenshot), paste penanda di sana, kembali ke
   Source: **byte dokumen tidak berubah** — ini oracle bahwa Preview benar-benar
   aktif dan read-only, bukan sekadar "tidak crash";
3. Preview → Split → Source (screenshot Split): dokumen tetap byte-identik,
   title bersih, close native exit 0.

**Yang tidak dibuktikan GUI:** piksel. Screenshot disimpan sebagai bukti tanpa
assertion piksel, jadi "gambarnya terlihat benar" **tidak** diklaim. Yang diuji
adalah gerbang path/ukuran/format (Rust) dan kenyataan bahwa dokumen berisi
gambar lokal + remote + hilang + kebesaran dirender di Preview dan Split tanpa
mengubah dokumen atau mematikan aplikasi.

## Execution status

[CI 39947223034](https://github.com/Indrawan007/Hylite/actions/runs/39947223034),
`3652184`, job `120392715915` **SUCCESS**, 23 Oktober 2026 11:36:11–11:48:45 UTC /
**19:36:11–19:48:45 Makassar**: format, strict core, Clippy `-D warnings`, build,
**988 Cargo**, **26 suite / 488 pemeriksaan** (444 clipboard, 17 title, 27 exit
native), dan upload artifact lulus.

Artifact `12499286571`, **3.785.113 bytes**, expires 2026-10-30T11:48:39Z, digest
SHA256 dilaporkan API
`68155a0f1626e6dfdcdc77ff4a1e5f6bb8cd78e49b379a4346de0a57e7a68208`
(zip tidak dapat diunduh dari sandbox ini).

Dua kegagalan CI sebelum hijau: satu argumen `base_dir` yang belum diteruskan ke
`Kind::Item` (kompilasi), dan `assertions_on_constants` dari Clippy pada tes
batas tampilan (tes itu dihapus, bukan di-`allow`). Ditambah satu tes model saya
sendiri yang salah mengasumsikan bentuk bersarang list/quote — diperbaiki dengan
menelusuri proyeksi, bukan mengasumsikan indeks.

## Manual / limits

- [ ] Gambar HiDPI, warna, dan proporsi: tidak diperiksa piksel sama sekali.
- [ ] GIF animasi, JPEG progresif, dan file rusak yang lolos header.
- [ ] Folder besar berisi banyak gambar: belum ada cache/limit per dokumen —
  setiap gambar yang lolos gerbang akan di-dekode oleh renderer.
- [ ] Path dengan `..` keluar folder dan symlink: dinyatakan, belum dibatasi.
- [ ] Arch/Hyprland/Wayland/portal/IME/GPU. X11 tiny-skia bukan proof target.
