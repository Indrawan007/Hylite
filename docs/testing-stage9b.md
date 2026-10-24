# Stage 9b — probe kelayakan hit-testing dan selection (NO-GO)

**Tanggal:** 24 Oktober 2026 · **Branch:** `arena/21a4c00b-hylite` · **Kode yang diuji:** `0599141`  
**CI:** [run 40058088359](https://github.com/Indrawan007/Hylite/actions/runs/40058088359), job `120765424636` — **workflow sukses; verdict probe NO-GO**.  
**Artifact GUI:** [`gui-x11-smoke`, id `12546617100`](https://github.com/Indrawan007/Hylite/actions/runs/40058088359/artifacts/12546617100), 4.779.552 byte, kedaluwarsa `2026-10-31T03:37:53Z`.

## Kesimpulan terbatas

Probe opt-in dan read-only berjalan pada aplikasi native Iced di X11 + Xvfb +
tiny-skia. Fokus editor dapat dibuktikan lewat marker caret setelah input
keyboard, dan `Home` mengembalikan caret ke offset 0. Namun, **325 koordinat
client-x yang memiliki pasangan sampel pada sweep maju dan balik semuanya
melaporkan caret 0**. Sweep tidak mengamati offset 1–11. Dense pass pada band
editor `x=256..511` juga tidak menemukan caret selain 0. Karena posisi caret 3
dan 7 tidak dapat ditemukan, harness tidak mengirim drag selection.

Hasilnya **NO-GO untuk kelayakan yang diuji**: bukti GUI ini tidak menunjukkan
hit-testing mouse-ke-glyph yang dapat dipakai secara andal, dan tidak
menghasilkan bukti anchor/arah selection. Selection bukan dinilai telah gagal
secara terpisah—aksi drag memang tidak dijalankan setelah titik endpoint tidak
teridentifikasi. Ini kesimpulan pada fixture, jalur input, dan konfigurasi
renderer yang dibatasi; bukan klaim bahwa semua versi Iced atau semua backend
mustahil melakukan hit-testing.

Pengamanan read-only berhasil: Source tetap byte-identik, file fixture di disk
tidak berubah, dokumen tetap clean, dan jendela keluar secara natural. Status
CI **success** berarti harness, immutability, lifecycle, serta semua gate yang
dijalankan lulus; status itu tidak mengubah verdict feasibility NO-GO menjadi
GO.

## Batas probe dan oracle

- Hanya berlaku untuk fixture `ab**CDEFGH**ijk\n` dan teks tampak unik-ASCII
  `abCDEFGHijk` dalam satu paragraf root berformat.
- Probe memakai widget native Iced `text_editor`; judul diagnostik menampilkan
  caret serta teks selection. Untuk selection, anchor/focus hanya akan
  diinferensikan dari teks terpilih yang unik ditambah posisi caret. Ini
  instrumentation eksperimen, bukan pemetaan Source produksi.
- Sweep maju memakai titik client-x kasar berjarak 8 px, memperhalus interval
  yang menunjukkan perubahan caret, lalu memakai dense pass terbatas pada
  256-pixel band awal area editor jika cakupan offset belum lengkap. Sweep balik
  mengulang koordinat yang mendapat event pada sweep maju. Penunjuk disinkronkan
  sebelum tiap klik.
- Drag maju dan balik masing-masing seharusnya memilih offset `3..7`, lalu
  setiap arah diulang. Karena caret 3 dan 7 tidak teramati, tidak ada drag yang
  dikirim dan tidak ada anchor, focus, atau arah selection yang dapat
  dilaporkan.
- Source dan file fixture tidak diedit oleh GUI. Screenshot adalah bukti
  pendamping saja, **bukan** oracle piksel.

## Tes otomatis — bukan bukti mouse GUI

Tes controller/widget memeriksa secara terpisah bahwa probe memerlukan opt-in,
aksi teks ditolak, Source/history tidak berubah, dan diagnostik teks ASCII unik
bisa membedakan selection maju maupun mundur saat diberikan aksi `Move`/`Select`
secara langsung. Itu memvalidasi logika diagnostik, bukan posisi klik/drag native.
Helper Python menguji parser marker, klasifikasi monotone/repeatability, serta
fixture; **93 tes Python** lulus. Hanya run GUI di atas yang menjadi bukti tentang
mouse native, dan run itu tetap NO-GO.

## Reproduksi

Suite menjalankan executable native pada Xvfb privat dan memilih fixture lewat
jalur opt-in khusus:

```sh
./scripts/test-gui-x11.sh --suite inline-probe \
  --artifacts /tmp/hylite-stage9b
```

Di CI, Stage 9b adalah step terpisah setelah suite Stage 9a:
`./scripts/ci-cargo.sh ./scripts/test-gui-x11.sh --suite inline-probe ...`.
Suite menambahkan **7 pemeriksaan keselamatan/harness**. Kesimpulan NO-GO
merupakan hasil data yang sah, bukan alasan untuk menonaktifkan atau melonggarkan
assertion; syarat Source tidak berubah dan natural close tetap wajib.

[CI 40054603403](https://github.com/Indrawan007/Hylite/actions/runs/40054603403)
lebih dahulu melewati batas waktu harness saat sweep reverse lama yang
memeriksa setiap piksel lebar jendela. Timeout itu bukan verdict. Sampler
kemudian dibatasi ke sweep kasar/refinement/band aktif, lalu run final
40058088359 selesai dan menghasilkan bukti NO-GO di atas.

## Gate setelah hasil

Stage 9b sebagai **feasibility probe** ditutup dengan hasil NO-GO; tidak ada
implementasi mapping caret/selection, dukungan drag umum, pengeditan Source dari
Preview, atau klaim WYSIWYG yang mengikuti hasil ini. Stage 9c/9d tidak dimulai
oleh probe ini. Sesudah penutupan Stage 9b, probe terpisah untuk custom Iced
widget/paragraph hit-test diimplementasikan sebagai
alat opt-in yang read-only. Hasilnya NO-GO terbatas pada fixture/kombinasi
X11 + Xvfb + tiny-skia yang baru itu; hasil tersebut tidak menimpa atau membuka
kembali bukti Stage 9b di atas. Lihat [kontrak, hasil, dan batas probe
alternatif](proposal-alternative-widget-probe.md). Jalur panel draft serta
seluruh gate lama tidak berubah.

Tidak terbukti di sini: Unicode, RTL/IME, glyph kompleks, scroll, drag di luar
satu baris, renderer selain tiny-skia, GPU/HiDPI, portal, Arch + Hyprland/Wayland,
performa, atau penggunaan pada dokumen nyata.
