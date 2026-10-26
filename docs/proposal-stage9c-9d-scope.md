# Keputusan scope — Stage 9c/9d

**Status: pilihan A disetujui pengguna — Stage 9c/9d ditunda.** Tidak ada
perubahan editor produksi. Dokumen ini merangkum keputusan setelah dua probe
hit-testing berakhir NO-GO pada cakupan terbatas; penundaan ini bukan klaim bahwa
Iced/backend mustahil dan bukan izin untuk mengimplementasikan marker hiding,
pemetaan Source per glyph, atau editing inline produksi.

**Basis bukti:** Stage 9a (CI `40039268185`), Stage 9b (CI `40058088359`), probe
custom-widget alternatif (CI `40070349616`), dan benchmark Stage 10b
([metodologi serta angka mentah](performance.md)). Semua bukti GUI berasal dari
X11 + Xvfb + tiny-skia; belum ada bukti Arch + Hyprland/Wayland.

## 1. Batas produk saat ini

- Stage 9a hanya membuktikan gerak caret **keyboard ASCII** dan satu splice
  Source/Undo/Redo byte-exact pada satu paragraf root satu baris. Itu bukan bukti
  hit-testing mouse per glyph atau selection.
- Stage 9b mengamati caret `[0]` pada 325 posisi x yang berulang; offset 3 dan 7
  tidak ditemukan, jadi drag tidak dijalankan.
- Probe custom-widget terpisah mencapai offset 0–6 pada 113 sampel x yang
  berulang, dengan kedua lintasan non-monotone. Offset 7–11 tidak ditemukan;
  drag tidak dijalankan. Ini **NO-GO terbatas untuk fixture/jalur tersebut**,
  bukan klaim universal tentang Iced atau backend lain.
- Preview default tetap memakai model block-level. Operasi blok yang sudah ada
  serta panel draft paragraf/sel tetap jalur editing terbatas; eksperimen 9a/9b
  dan probe alternatif tidak menggantikan jalur itu.

## 2. Pilihan scope

| Pilihan | Scope | Konsekuensi | Rekomendasi |
|---|---|---|---|
| **A — Tunda inline editing (dipilih)** | Stage 9c/9d tidak dikerjakan untuk saat ini. Tidak menyembunyikan marker; pertahankan Preview dan panel draft/operasi blok yang ada. Tutup hanya dua feasibility probe pada cakupannya, tanpa menyatakan Iced/backend mustahil. | Tidak ada perubahan editor produksi untuk 9c/9d. Jika inline editing kelak menjadi tujuan lagi, buka keputusan baru dengan gate feasibility terpisah. Stage 10a telah dipisahkan dan diimplementasikan di bawah izin scope tersendiri. | Dipilih pengguna karena bukti pointer/selection belum memadai dan tidak meminta perluasan di luar persetujuan saat ini. |
| **B — Gate baru, lalu Stage 9c yang sangat terbatas** | Belum ada perubahan produksi sekarang. Pertama, user menyetujui eksperimen feasibility baru yang opt-in/test-only; jika gate lulus, ajukan keputusan implementasi terpisah. Kandidat produk awal hanya root paragraph satu baris dan subset inline yang sudah dikenal; heading/list/quote/tabel, penghapusan panel draft, Unicode umum, dan WYSIWYG luas tetap di luar scope awal. | Membutuhkan bukti baru yang benar-benar menguji event pointer/drag dan pemetaan teks; hasil dua probe lama tidak dapat dipakai sebagai GO. Keberhasilan gate tidak otomatis mengizinkan implementasi produksi. | Hanya jika produk memang perlu inline editing dan tersedia cakupan uji target yang disepakati. |
| **C — Perluasan Stage 9d** | Editing langsung untuk heading/list/quote/sel tabel sekaligus pensiun panel draft. | Risiko tertinggi: lebih banyak grammar/source mapping, selection, Undo, refocus dan migrasi UX harus dijamin. Tidak dibenarkan oleh bukti saat ini. | **Tidak direkomendasikan sekarang.** Pertimbangkan hanya setelah 9c punya bukti serta keputusan scope tersendiri. |

**Keputusan pengguna: A — defer 9c/9d.** Ini berarti menunda, bukan menyatakan
bahwa inline editing tidak mungkin. Pilihan B/C tetap membutuhkan keputusan dan
persetujuan eksplisit sebelum gate atau pekerjaan produk dimulai.

## 3. Gate minimum jika pilihan B kelak dipilih

Gate ini adalah rancangan awal, bukan acceptance test yang sudah disetujui.
Sebelum ada produksi, eksperimen terpisah harus:

1. Tetap opt-in, test-only/read-only, dan tidak mengubah Source, dirty state,
   history, file di disk, Preview default, maupun gate GUI lama.
2. Mengukur event pada widget dan `Paragraph` yang sama dengan yang dirender;
   tidak memakai sweep koordinat layar yang tidak terikat pada bounds aktual.
3. Menunjukkan hit-test berulang dan urut pada posisi teks yang terlihat,
   termasuk batas antar-run. Menguji klik maju/mundur serta drag dua arah dan
   anchor/focus; offset yang tidak ditemukan menghentikan tes, bukan dilewati
   dengan mengarang hasil.
4. Memisahkan indeks visible grapheme dari byte UTF-8 Source secara eksplisit.
   Bukti ASCII saja tidak mengizinkan klaim atau scope Unicode/RTL/IME.
5. Memiliki oracle Source byte-exact dan pemeriksaan stale target/snapshot,
   refusal no-op/unsupported, serta tidak menyimpulkan selection dari pencarian
   substring teks yang kebetulan unik.
6. Mendefinisikan platform gate sebelum dijalankan. CI X11 + Xvfb + tiny-skia
   tetap bukan bukti Arch + Hyprland/Wayland, GPU, IME, HiDPI, atau RTL.

Jika gate feasibility lulus, keluaran berikutnya adalah **permintaan persetujuan
implementasi produk** dengan scope, input yang didukung, fallback/refusal, dan
acceptance tests yang sudah dibatasi. Kelulusan probe saja bukan izin otomatis.

## 4. Ketergantungan yang tetap terbuka

- Stage 10a dapat berdiri sendiri dari 9d: fast path parser/cache Preview
  produksi telah diimplementasikan melalui persetujuan scope terpisah; full parse
  tetap oracle/fallback. Lihat [scope dan status Stage 10a](proposal-stage10a-scope.md).
  Keputusan ini tetap tidak mengizinkan perubahan Stage 9c/9d.
- Stage 10c viewport culling/scroll sync dan Stage 11a instrumentasi belum
  dimulai; keduanya dapat menyentuh aplikasi produksi dan tidak otomatis
  diizinkan oleh keputusan scope dokumen ini.
- Estimasi roadmap dan kesiapan rilis tetap terpisah. Tidak ada angka Stage 10b
  yang menjadi klaim kecepatan; baseline satu-run tetap hanya catatan lingkungan
  di [docs/performance.md](performance.md).

## 5. Keputusan

- **Pilihan yang disetujui pengguna:** A — defer Stage 9c/9d dan pertahankan Preview/panel draft saat ini.
- **Stage 9c/9d:** deferred; tidak ada marker hiding, mapping per glyph, atau pensiun panel draft.
- **Stage 10a:** fast path konservatif sudah diimplementasikan secara terpisah; scope, fallback, verifikasi, dan observasi Stage 10b ada di [proposal Stage 10a](proposal-stage10a-scope.md) dan [performance notes](performance.md).
- **Perubahan kode editor produksi untuk 9c/9d:** tidak diizinkan oleh keputusan ini.

## 6. Pembaruan scope — 26 Oktober 2026

Permintaan pengguna yang lebih baru, **“Implementasikan WYSIWYG umum ala
MarkText”**, secara eksplisit membuka kembali pekerjaan produk Stage 9c/9d dan
menggantikan keputusan defer pada bagian sebelumnya. Riwayat pilihan A tetap
benar untuk tanggal/keadaan saat itu, tetapi bukan batas scope yang berlaku
sekarang. Probe 9b dan custom-widget tetap NO-GO historis terbatas; hasilnya
**tidak** dipakai sebagai bukti hit-testing/selection produksi.

Irisan awal yang sedang dikerjakan memakai editor visual whole-block pada
Preview untuk paragraf root satu baris dan heading ATX root: teks hasil render
serta style inline diproyeksikan ke draft, ada format seleksi dan Undo/Redo lokal,
dan Apply melewati full parser preflight sebelum satu transaksi Source. Ini
menghindari klaim pemetaan caret/glyph ke Source. Scope tersebut **belum WYSIWYG
umum ala MarkText**; nested/list/quote/table, multiline, Setext, pemetaan glyph,
validasi desktop, serta paritas visual masih terbuka. Status kode dan verifikasi
harus diperbarui berdasarkan hasil implementasi terbaru, bukan dianggap selesai
dari catatan rencana ini.

Pembukaan kembali ini hanya mengotorisasi pekerjaan WYSIWYG/Stage 9c/9d. Stage
11b tetap menunggu validasi desktop nyata; Stage 11c/packaging tidak tercakup.
