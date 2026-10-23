# Stage 7i — konteks draft dan umpan balik aksi yang diblokir

Melanjutkan [hardening7h](testing-stage7h.md). Dua perubahan UX kecil yang tidak
menggeser satu pun koordinat kontrol:

1. Panel draft menampilkan **rentang baris Source** yang sedang diedit (dan posisi
   root di antara root multiline eligible).
2. Aksi yang selama ini **diblokir diam-diam** saat panel draft terbuka (Save, Save As,
   New, Open, Export, ganti mode, Undo/Redo Source, Find/Find Next, Go-to, sidebar,
   Zen, tutup jendela) kini memberi pesan status yang menyebut panel dan jalan
   keluarnya (`Apply` atau `Esc`).

Tidak ada aksi produk baru, tidak ada perubahan pada tombol/tinggi baris/koordinat,
tidak ada perubahan semantik Source, history, atau guard.

## Kontrak

- **Provenance bersifat baca-saja dan terautentikasi.** `Document::multiline_context`
  lebih dulu memvalidasi target warm exact7b (snapshot/ranges/payload) sebelum
  menghitung: rentang baris1-based inklusif dari payload di Source, plus indeks dan
  jumlah root multiline eligible pada proyeksi warm. `Document::paragraph_line`
  melakukan hal yang sama untuk draft paragraf satu baris.
- Offset byte UTF-8 diperiksa (`is_char_boundary`, batas teks); target dingin, forged,
  atau di luar batas menghasilkan `None` → panel memakai teks bantuan lama.
- Nilai provenance **disalin sekali saat sesi dibuka** (open/move/Apply & Next/
  Apply & Previous). Mengetik atau membuat draft tidak valid **tidak** menghitung ulang
  dari byte kandidat; panel tidak pernah mengklaim rentang yang belum ada di Source.
- Format satu baris tetap: `Source line 12 · root 2 of 5 · changes only on Apply` atau
  `Source lines 12–15 · root 2 of 5 · …`, dan untuk paragraf satu baris
  `Source line 12 · changes only on Apply · stays one root paragraph`. Tinggi baris
  tetap20px sehingga input, tombol, dan footer tidak bergeser.
- **Umpan balik blokir** hanya untuk gesture pengguna yang benar-benar akan bertindak
  pada dokumen. `EndTypingGroup`, reply asinkron (`FileSaved`/`HtmlExported`/
  `FileOpened`), dan ketikan tidak pernah menimpa status dengan pesan blokir. Pesan
  bernada sama untuk ketiga panel: `A multiline paragraph draft is open: Apply or
  press Esc to Cancel before another action` (variasi `A paragraph draft` /
  `A cell draft`).
- Blokir itu sendiri tidak berubah: tidak ada operasi file yang dimulai, mode tidak
  berganti, jendela tidak menutup, snapshot/history/draf tetap utuh.

## Implementasi

`state.rs` menambah dua helper baca-saja. `ui::multiline_paragraph::Draft` mendapat
`context: Option<(usize,usize,usize,usize)>` dengan `with_context`, dan
`ui::paragraph_edit::Draft` mendapat `line: Option<usize>`; keduanya default `None`
sehingga buffer verifikasi sementara7e/7f tidak perlu provenance. `provenance_line`
adalah fungsi murni sehingga formatnya bisa diuji tanpa render. `app::Hylite::
report_blocked_draft_action` dipanggil pada ketiga cabang guard draft di `update`.

**921 Cargo** =910sebelumnya +**11Rust** (9 controller/app, 1 UI multiline, 1 UI
paragraph); **294strict core** subset unchanged, **63Python**.

- App/controller: konteks benar di Preview & Split untuk root pertama dan setelah
  Next; memuat context tidak mengubah snapshot/render key/teks/caret; context
  diperbarui setelah Apply & Next (root 2/2) dan Apply & Previous (root 1/2) serta
  cocok dengan baris Source yang benar-benar ter-commit; context tidak berubah saat
  draft menjadi tidak valid; snapshot forged → `None`, dan helper paragraf menolak
  target multiline; draft paragraf satu baris melaporkan `line`1.
- Blokir: enam gesture (Save/SwitchMode/New/Export/Undo/ToggleSidebar) memberi pesan
  yang menyebut panel + `Esc` tanpa mengubah snapshot/mode/operasi/judul/draf; close
  request tidak meninggalkan pending action dan tidak menutup draft; panel paragraf
  dan sel memberi pesan untuk panelnya masing-masing; reply asinkron dan
  `EndTypingGroup` **tidak** menimpa status.
- UI: format provenance satu baris vs rentang, fallback tanpa konteks, dan
  affordance Apply tidak berubah oleh konteks.

## GUI

Tidak ada suite baru: perilaku produk yang berubah hanya teks header/status, dan
semua kontrol tetap pada koordinat yang sama. Ke-20 suite (**394 checks**) dijalankan
ulang utuh sebagai regresi dan lulus; 28 badan exercise lama tetap AST-identical
`bb37223`, tanpa perubahan bytes/koordinat/oracle/deadline/refocus.

## Execution status

Local fmt/harnessfmt/294strict core/63Python/diff/AST check PASS.

[CI 39901301577](https://github.com/Indrawan007/Hylite/actions/runs/39901301577),
`d22c7f30a773cd44962aecce24305b0830ae4eb5`, job120243267024 **SUCCESS**,
2026-10-23 **04:37:25–04:47:53 UTC** / **12:37:25–12:47:53 Makassar**. Format,
strict core subset, Clippy, build, **921Cargo**, seluruh **20suite** (**394GUI**)
dan upload artifact lulus pada percobaan pertama. Artifact `12479306282`,
**3.238.614bytes**, SHA256
`60eeac0c449c98f69ae5e1ce94f142e3c169ff1a991dd32fdd1d711ae9787adb`,
expires 2026-10-30T04:47:49Z, unexpired; screenshot tidak direview visual.

## Manual / limits

- [ ]Apakah teks header yang lebih panjang terpotong pada lebar jendela kecil/narrow
  window, HiDPI, atau font non-default; status bar yang memuat pesan blokir panjang.
- [ ]Arch/Hyprland/Wayland/portal/IME/RTL/GPU. X11 tiny-skia bukan proof target.

Provenance hanya informatif: ia tidak menambah navigasi lintas jenis blok, tidak
membuat Apply implisit, dan tidak memperluas root eligible di luar7b. Semua jalur
tetap O(n) salin/parse; bukan klaim performa.
