# Stage 7g — shortcut commit eksplisit untuk draft multiline

Melanjutkan [Apply & Previous7f](testing-stage7f.md). Panel draft multiline kini
punya shortcut eksplisit untuk aksi yang sudah ada; tidak ada aksi produk baru,
tidak ada Apply implisit, dan tidak ada perubahan semantik Source/history.

## Kontrak

- **Ctrl+Enter** = Apply (commit satu edge, tutup draft).
- **Ctrl+PageDown** = Apply & Next, **Ctrl+PageUp** = Apply & Previous.
- Hanya **Ctrl persis** yang diterima. `Ctrl+Shift`, `Ctrl+Alt` (AltGr), Logo
  (compositor), Shift saja, maupun tanpa modifier **tidak** memicu commit; Enter
  tanpa modifier tetap newline native dan PageUp/PageDown tanpa modifier tetap
  navigasi kursor native.
- Shortcut hanya aktif saat editor draft berstatus *focused*. Saat status lain
  (mis. `Active`) binding kembali `None`, sehingga tidak ada commit liar.
- Shortcut memakai **message yang sama** dengan tombol, jadi seluruh guard7d–7f
  berlaku identik: draft error/invalid/oversize, target stale, boundary pertama/
  terakhir, unchanged move, mode Source, prompt pending, operasi file aktif, dan
  modal lain tetap menolak tanpa mengubah Source atau membuang history lokal.
  Boundary/refusal hanya menampilkan notice/error dan mempertahankan draft + fokus.
- Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y pada draft tetap menjadi **history lokal**7c,
  tidak berubah; Ctrl+S, Ctrl+O, Ctrl+N, Ctrl+F, Ctrl+G, Ctrl+Shift+M dan shortcut
  aplikasi lain tetap tidak diambil alih panel draft.
- Tidak ada shortcut baru di Source, panel sel tabel, panel paragraf satu baris,
  atau level aplikasi. Tidak ada perubahan tata letak tombol; hanya teks bantuan
  footer yang kini menyebut shortcut.
- Karena shortcut tidak melewati tombol, ia juga tidak menambah edge history,
  tidak mengubah caret/checkpoint/cache, dan tidak membuat Apply & Next/Previous
  menjadi implisit.

## Implementasi

`commands::draft_commit_shortcut(key, modifiers) -> Option<Message>` adalah satu
fungsi murni: menolak semua kombinasi selain `Modifiers::CTRL` persis, lalu memetakan
`Named::Enter`/`PageUp`/`PageDown` ke `ApplyMultilineParagraph` /
`ApplyMultilineParagraphPrevious` / `ApplyMultilineParagraphNext`.
`ui::multiline_paragraph::session_key_binding` memanggilnya untuk status *Focused*
setelah jalur history lokal, dan membungkusnya sebagai `text_editor::Binding::Custom`
sehingga Iced tidak menyisipkan karakter dan subscription global tidak ikut
menafsirkan tombol tersebut.

**905 Cargo** =903sebelumnya +**2Rust** (1 tes murni `commands`, 1 tes UI binding);
**294strict core** subset unchanged, **63Python** =61+2.

- Helper: ketiga tombol memetakan ke message yang benar; enam kombinasi modifier
  ditolak per tombol; `Ctrl+Z`/`Ctrl+S` tidak pernah menjadi commit.
- UI binding: matrix focused → Custom Apply/Previous/Next; AltGr/Logo/Shift/plain
  ditolak; status `Active` → `None`; Enter tanpa modifier **bukan** `Custom`
  (newline native); capture history lokal tetap utuh.

## Private GUI suite kedua puluh

`--suite paragraph-draft-shortcuts`: private app/Xvfb/XDG/env/clipboard, alarm180s,
step5menit sendiri, job45menit tetap, tanpa retry/runtime hook. **28old exercise
bodies AST-identical `bb37223`**, 19gate lama dipertahankan; private DISPLAY guard
kini20suite.

**16checks** =15Copy(5Source+10draft)+natural clean Close0:

1. Welcome → tempel fixture dua root raw Unicode → Preview.
2. Buka root pertama: payload raw exact + fokus native; paste tiga baris Unicode
   tetap lokal; **Ctrl+Enter** menutup draft dan satu Source Undo/Redo membuktikan
   hanya satu edge atomik.
3. Split: buka root pertama yang sudah diterapkan; **Ctrl+PageDown** membuka root
   kedua tanpa mengubah Source; **Ctrl+PageUp** kembali ke root pertama dengan
   offset baru; Ctrl+PageUp lagi di boundary pertama hanya notice (tanpa wrap).
4. Ctrl+PageDown ke root terakhir; draft changed lalu **Ctrl+PageDown** di boundary
   terakhir menolak dan mempertahankan draft; Escape membatalkan tanpa menyentuh
   Source yang sudah diterapkan.
5. Dua Source Undo bertahap kembali ke fixture lalu welcome bersih; Close0.

Dua klik hanya produk Open (Preview dan Split). Copy dilakukan **sebelum** klik
disabled manapun; setelah Ctrl+Enter yang menutup panel, fokus Source dipulihkan
lewat mode switch seperti suite7b, bukan workaround refocus.

```sh
./scripts/test-gui-x11.sh --suite paragraph-draft-shortcuts --artifacts "$(mktemp -d)"
# Keep all19old suites too, never substitute.
```

## Execution status

Local fmt/harnessfmt/294strict core/63Python/diff/AST check PASS.

[CI 39895925701](https://github.com/Indrawan007/Hylite/actions/runs/39895925701),
`0539b029a8a2db0abece1750b68784e7acedf0cc`, job120226246206 **SUCCESS**,
2026-10-23 **03:36:36–03:47:19 UTC** / **11:36:36–11:47:19 Makassar**. Format,
strict core subset, Clippy, build, **905Cargo**, seluruh **20suite** (**394GUI**,
16checks suite baru) dan upload artifact lulus. Artifact `12476433906`,
**3.253.935bytes**, SHA256
`aa7def659290d7b336bc7b0860bbea4db9396b1968e4349391789539d294a138`,
expires 2026-10-30T03:47:13Z, unexpired. Native counts dari check annotations;
screenshot tidak dibuat suite ini dan full archive tidak diperiksa.

Iterasi CI sebelum lulus (produk `faa1c74` tidak berubah untuk meloloskan tes):
`39894389337` gagal karena tes helper membandingkan `Option<Message>` (Message
tidak `PartialEq`); `39894782748` gagal `clippy::clone_on_copy` (`Named` adalah
`Copy`); `39894936977` suite GUI berhenti di Copy pertama setelah Ctrl+Enter
karena fokus Source belum pulih pada keypress berikutnya. Perbaikan ketiganya
test-only, tanpa melonggarkan lint/oracle/byte/koordinat/deadline.

Catatan: batas capture `text_input` yang membuat shortcut ini khusus draft
`text_editor` dijelaskan di [7h](testing-stage7h.md).

## Manual / limits

- [ ]Konflik shortcut nyata di Hyprland (compositor dapat mengambil Ctrl+Enter
  atau Ctrl+PageUp/PageDown sebelum aplikasi), IME, keyboard non-US, dan
  aksesibilitas keyboard-only.
- [ ]Arch/Hyprland/Wayland/portal/RTL/HiDPI/GPU. X11 tiny-skia bukan proof target.

Tidak ada perubahan pada tombol, tata letak, Apply implisit, shortcut Source/panel
sel, atau traversal lintas jenis blok. Apply & Next/Previous tetap hanya untuk
paragraf multiline root eligible; batas 2–128baris/16KiB dan semua guard
Source/file/modal tidak berubah.
