# Stage 6i — navigasi panel draft sel dan Apply & Next

Successor [raw cell draft 6h](testing-stage6h.md). CI akhir 6h yang sebelumnya
masih antre kini [success pada f69d044](https://github.com/Indrawan007/Hylite/actions/runs/39795536536).
Ini navigasi antarpayload kanonis dalam panel, bukan inline WYSIWYG/spreadsheet.

## Kontrak produk

- **Previous / Next** mengunjungi sel real tabel yang sama secara **row-major**.
  Row0 header, body mulai1, kolom tampil mulai1. Boleh melintasi batas header/body;
  tidak wrap dari akhir ke awal, tidak lompat tabel, tidak membuat row baru.
- Hanya draft **valid dan persis sama dengan payload Source saat ini** yang boleh
  pindah tanpa Apply. Draft berubah/invalid tetap dipertahankan. Notice navigasi
  terpisah dari validation error, sehingga penolakan dirty Next tidak mematikan
  Apply yang valid. Tidak auto-save atau auto-discard saat Previous/Next.
- **Apply & Next** adalah tindakan eksplisit: satu payload diterapkan sebagai
  satu Atomic `PreviewTable`, lalu panel membuka raw payload sel berikutnya.
  Source caret tetap di akhir payload yang diterapkan, bukan glyph/sel berikutnya.
  Navigasi read-only tidak mengubah Source caret/selection/checkpoint/revision.
- Di sel terakhir, Next dan Apply & Next disabled; forged command juga menolak
  tanpa menerapkan perubahan. Gunakan Apply lama untuk menyelesaikan sel terakhir.
  No-op Apply & Next pindah tanpa revision/history/redo clearing.
- **Cancel/Escape hanya membuang draft saat ini**. Apply sebelumnya dalam rangkaian
  tetap ada di Source dan masing-masing bisa di-Undo setelah menutup panel.
  Enter/Apply lama masih menerapkan satu sel dan menutup, tanpa shortcut baru.
- Semua batas/provenance 6h tetap: root canonical rectangular edge-pipe table,
  real nonpadding cells, 12 kolom/128 row/512 sel, payload single-line16KiB,
  escaped pipe/backslash parity/control rules, outer whitespace/raw bytes utuh,
  cache warm exact snapshot/table/index/dimension/payload membership.
- Resolver neighbor portable hanya menghitung posisi bounded; bukan bukti auth.
  Document mengautentikasi current target sebelum Previous/Next/Apply & Next.
  Cold/stale/forged/partial target tidak rebase dan tidak mengubah Source/draft.
- Untuk Apply & Next efektif, **candidate Source dan proyeksi parser baru
  dipreflight sebelum commit**. Root table start/dimensions/real next-cell metadata
  harus cocok. Kegagalan menolak seluruh edit, bukan commit lalu memakai offset
  lama. Sesudah satu commit, cache candidate dipasang untuk revisi baru dan
  next target distamp full Snapshot baru. Source offset diselesaikan ulang,
  termasuk setelah perubahan panjang UTF-8/header/payload.
- Logical focus tetap pada draft input saat berpindah. Apply/Cancel terakhir
  memulihkan Source/query context 6h, mode tetap, search refresh hanya pada commit.
  Modal file/navigation/history guard, inflight save/export ack, dirty checkpoint,
  Source barrier dan history budget200entries/8MiB tetap. Tidak menambah izin stack.
- Window title menampilkan `[cell draft] [cell R:C]` sebagai status pengguna.
  Panel kini top-aligned dengan input/navigation rows berukuran tetap agar footer/
  error wrapping tidak menggeser kontrol. Main Preview/Split bar/picker dan semua
  koordinat suite lama tidak berubah. Tidak ada glyph/caret map atau scroll sync.

## Tes

**21 Rust baru**, target total Cargo **632** (hasil aktual di bawah):

- **5 portable neighbor**: row-major/header/body, no-wrap bounds, invalid/overflow/
  caps, traversal+inverse seluruh grid sampai cap, auth counterexample.
- **6 Document**: navigation tanpa mutasi/caret/checkpoint; preflight fresh UTF-8/
  header offsets dan separate Atomic Undo; no-op/redo; cold/forged/stale/invalid/
  last-cell reject-all; stale after Source barrier/Undo; save checkpoint/branch.
- **8 controller**: valid traversal/focus context/title, dirty refusal without
  poisoning Apply, Apply & Next/Cancel-current-only, no-op/bounds/redo, invalid/
  stale retention, modal file/mode/history guards+save ack, sequential atomic
  edits/Find refresh/context, no-draft messages inert.
- **2 UI adapter**: bounds/validation disable rules dan navigation notice terpisah
  dari valid Apply. Semua tes 6h lama tetap tanpa pelemahan expected values.

Core **231** (226 lama + 5 neighbor) subset Cargo, bukan tes unik tambahan.
Python **35** (33 lama + 2 navigation helpers) mengunci literal sentinel Source,
raw draft copies/action sequence/no-refocus/close dan dispatch eksklusif.
Guard private DISPLAY mencakup enam suite sebelum subprocess.

## Suite GUI privat keenam

Gate lama **116 regression + 9 batch + 9 alignment + 9 rows + 14 cells** tetap
utuh. Semua 12 phase regression dan empat body suite independen lama diverifikasi
sama secara AST terhadap Stage6h. `--suite cell-navigation` memiliki app/Xvfb/
XDG/env/clipboard baru, alarm180s dan step5min sendiri. Job45min tetap; tidak ada
retry/runtime hook/Source refocus workaround/relaksasi expected bytes gate lama.

Navigation aktual **16 checks** = 15 exact Copy (8 Source + 7 draft) + 1 natural
clean exit:

1. Welcome/fixture canonical table dengan UTF-8/markup/escaped pipe/entity/tab.
2. Previous body→header dan Next header→body memuat raw payload dengan native focus.
3. Dirty Next menolak dan menyimpan teks; koreksi Unicode tetap bisa Apply & Next.
4. Next payload raw di-resolve ulang setelah commit dan perubahan byte length.
5. Last-cell Next/Apply & Next disabled; Previous kembali ke applied payload.
6. No-op Apply & Next membuka raw neighbor tanpa record tambahan.
7. Edit draft berikut lalu Escape: hanya draft itu dibuang, prior commit tetap.
8. Satu Preview Undo/Redo mengembalikan exact Source.
9. Split Apply & Next, Escape, lalu X: Source focus/caret pada applied payload end.
10. Source typing dan commit terpisah; Undo ke welcome clean tanpa extra steps.
11. Client Close exit alami0, bukan kill sebagai oracle.

13 klik hanya picker/opsi/navigation produk, Copy semuanya `refocus=False`.
Title posisi sel adalah UI status, bukan test hook. Evidence di `cell-navigation/`,
`cell-navigation-draft.png` / `cell-navigation-split.png` bukan pixel oracle.

```sh
./scripts/test-gui-x11.sh --suite cell-navigation --artifacts "$(mktemp -d)"
# Wajib pula jalankan semua lima suite lama, bukan substitusi.
```

## Status eksekusi

Lokal: format Rust, **231 strict core**, **35 Python**, `git diff --check` lulus.
[CI 39863457232](https://github.com/Indrawan007/Hylite/actions/runs/39863457232)
pada `983b517e41407fda69432d54851d127e648e867c` **success**;
`gh run watch --exit-status` keluar0. Job `120119664548`: format/build debug/
Clippy strict, **632 Cargo**, **231 strict core** (subset), **35 Python**,
serta **116+9+9+9+14+16 GUI** lulus. Annotation Check-run memuat semua16 PASS
navigasi, 14 PASS draft lama dan hasil suite lama tanpa perubahan oracle.
Total **173 GUI** = 154 clipboard (145 Source + 9 draft input), 12 title,
7 exit alami. Artifact `12462558973`, `gui-x11-smoke`, 1.975.135 bytes,
belum expired saat diperiksa; screenshot belum direview visual. Native CI
berjalan 22 Oktober UTC, tanggal lokal Makassar **23 Oktober2026**.

## Batas/manual

- [ ] Query/Go-to physical focus setelah navigasi, header Apply & Next/EOF UI.
- [ ] Semua menu/keyboard traversal, 16KiB/Tab/IME/RTL/accessibility/HiDPI.
- [ ] Glyph/caret/scroll visual, Arch/Hyprland/Wayland/portal/GPU.

Tidak ada implicit Apply/auto-add row, multi-cell draft/selection/session Undo,
inline paragraph/list/cell editing, rich selection formatting, glyph↔caret mapping,
selection/scroll sync, incremental parse/virtualisasi atau benchmark speed/RAM.
Preflight menambah full parse O(n), bukan speedup. Gutter/LF canonical dan startup
6b exit101 yang belum terdiagnosis tetap; suite baru bukan klaim fix startup.
