# Verifikasi build dan tes native — 23 Oktober 2026

## Stage 6n terbaru — bounded raw column moves with alignment

[CI 39874500418](https://github.com/Indrawan007/Hylite/actions/runs/39874500418),
`46c89c632bef97018ca8633a4877569fe1528408`, **success**, watch exit0,
job `120156694687`,22 Oktober2026 23:51:38–23:59:38 UTC /23 Oktober Makassar.
Format/strict Clippy/native debug build, **738 Cargo**, **264 strict core**
(subset), **45 Python**, sebelas suite Xvfb **116+9+9+9+14+16+11+11+11+11+11
GUI** lulus. Total **228 GUI** =204 clipboard (195 Source +9 draft),12 title,
12 natural exits. Artifact `12468122247`,2.433.374 bytes, belum expired saat
diperiksa; screenshot belum direview visual.

Column suite membuktikan Preview Move right dan Split Move left menukar raw
header/delimiter/body serentak, alignment/dash/Unicode/markup/escaped pipe/entity/
indent/tab/prefix/suffix utuh, one-step Preview Undo/Redo, Source focus/caret di
selected moved header payload lewat native X tanpa refocus, separate typing/move/
paste history dan natural Close0.22 Rust baru mencakup bounded neighbors, entire
raw-column no-op preserving redo/checkpoint/cache, cold/stale/forged/nested/ragged/
noncanonical/EOF/fallback/growth caps, header-only/empty/indent, parser preflight,
fresh moved cell edit, save/modal/file/search/logical Go-to context.

Semua21 old exercise bodies AST-identical; tidak melonggarkan oracle/koordinat/
deadline/refocus. Runner kesebelas tetap alarm180s/step5min dan job45min. CI docs6m
39873812558 cancelled saat push6n supersede; feature6m tetap success dan semua
old gates kembali lulus di6n. Full parse/reconstruction/history O(n), bukan speedup.
Tidak ada drag/bulk/resize/multi-column selection atau inline WYSIWYG. Middle/
3+kolom/header-only/empty/12kolom/3spasi UI, physical no-op/query/Go-to focus,
large menu keyboard/scroll dan Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU tetap
manual. [Kontrak dan batas Stage6n](testing-stage6n.md).

## Stage 6m — bounded raw body-row moves

[CI 39872957659](https://github.com/Indrawan007/Hylite/actions/runs/39872957659),
`27da9ae2fa9238324a65abae930bed1fd819b759`, **success**, watch exit0,
job `120151569350`,22 Oktober2026 23:37:02–23:43:37 UTC /23 Oktober Makassar.
Format/strict Clippy/native debug build, **716 Cargo**, **257 strict core**
(subset), **43 Python**, sepuluh suite Xvfb **116+9+9+9+14+16+11+11+11+11 GUI**
lulus. Total **217 GUI** =194 clipboard (185 Source +9 draft),12 title,11 natural
exits. Artifact `12467486534`,2.335.915 bytes, belum expired saat diperiksa;
screenshot belum direview visual.

Move suite membuktikan Preview Move down dan Split Move up menukar whole raw
body lines, UTF-8/markup/escaped pipe/entity/indent/ragged/header/suffix/tab utuh,
one-step Preview Undo/Redo, Source focus/caret di moved physical row start lewat
native X tanpa refocus, separate typing/move/paste history dan natural Close0.
22 Rust baru mencakup no-wrap/header bounds, identical raw no-op/redo preservation,
cold/stale/forged/nested/literal/EOF/fallback/growth caps, parser preflight, fresh
moved-cell edit, save/checkpoint/modal/file/search/logical Go-to context.

Semua20 old exercise bodies AST-identical; tidak melonggarkan oracle/koordinat/
deadline/refocus. Runner kesepuluh tetap alarm180s/step5min dan job45min. CI docs6l
39872209785 juga success. Full parse/reconstruction/history O(n), bukan speedup.
Tidak ada header/column movement, drag/bulk/multi-row selection atau inline
WYSIWYG. Middle/3+body/1kolom/12kolom/3spasi UI, physical no-op/query/Go-to focus,
large menu keyboard/scroll dan Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU tetap
manual. [Kontrak dan batas Stage6m](testing-stage6m.md).

## Stage 6l — explicit atomic column deletion

[CI 39871210748](https://github.com/Indrawan007/Hylite/actions/runs/39871210748),
`492669883a071d59258c94e8f329c3f273148eea`, **success**, watch exit0,
job `120145818542`,22 Oktober2026 23:21:18–23:28:51 UTC /23 Oktober Makassar.
Format/strict Clippy/native debug build, **694 Cargo**, **250 strict core**
(subset), **41 Python**, sembilan suite Xvfb **116+9+9+9+14+16+11+11+11 GUI**
lulus. Total **206 GUI** =184 clipboard (175 Source +9 draft),12 title,10 natural
exits. Artifact `12466407895`,2.237.302 bytes, belum expired saat diperiksa;
screenshot belum direview visual.

Delete suite membuktikan first/last column header/delimiter/body dipotong serentak,
retained raw bytes/alignment/UTF-8/markup/escaped pipe/entity/tab utuh, one-step
Preview Undo/Redo, Split surviving header payload caret/focus lewat native X tanpa
Source refocus, distinct typing/delete/paste history dan natural Close0.22 Rust
baru mencakup last-column protection, cold/stale/forged/ragged/literal/EOF/caps,
header-only/indent/middle, candidate preflight, retained fresh cell edit, save/
checkpoint/modal/file/search/Go-to context. Last-column menu absence bukan pixel
oracle, protection diuji model/Document/controller.

Semua19 old exercise bodies AST-identical; tidak melonggarkan oracle/koordinat/
deadline/refocus. Runner kesembilan tetap alarm180s/step5min, job45min tetap.
CI docs6k39868311414 sebelumnya juga success. Reconstruction/history +full parse
preflight O(n), bukan speedup. Tidak ada last-column/whole-table deletion,
reorder/resize/inline typing. Middle/header-only/empty/12kolom UI, menu keyboard/
scroll, active Find/Go-to physical focus dan Arch/Hyprland/Wayland/portal/IME/RTL/
HiDPI/GPU tetap manual. [Kontrak dan batas Stage6l](testing-stage6l.md).

## Stage 6k — explicit atomic blank columns

[CI 39867392194](https://github.com/Indrawan007/Hylite/actions/runs/39867392194),
`9b1e931a2293120bcc50b3655424e90c7f4cfe18`, **success**, watch exit0,
job `120133186140`,22 Oktober2026 22:47:35–22:53:50 UTC /23 Oktober Makassar.
Format/strict Clippy/native debug build, **672 Cargo**, **243 strict core**
(subset), **39 Python**, delapan suite Xvfb **116+9+9+9+14+16+11+11 GUI** lulus.
Total **195 GUI** =174 clipboard (165 Source +9 draft),12 title,9 natural exits.

New column suite membuktikan blank header/delimiter/body ditambahkan serentak,
raw UTF-8/markup/escaped pipes/entity/spacing/old alignment/tab tetap, one-step
Preview Undo/Redo, Split Source caret/focus di new header via native X tanpa
refocus, separate typing/insert/paste history sampai welcome clean dan natural
close0.23 Rust baru mencakup root/cold/stale/forged/ragged/caps, candidate parser
preflight, header-only/indent, fresh body cell edit, modal/file/save/checkpoint/
search/Go-to context. Semua18 old exercise bodies AST-identical; tidak ada
relaksasi oracle/koordinat/deadline/refocus. Tiap runner alarm180s/step5min,
job45min tetap. Artifact `12464787718`,2.143.016 bytes, belum expired saat
diperiksa; screenshot belum direview visual. CI docs6j `39866562690` cancelled
saat push6k supersede; feature6j tetap green, gate6j kembali lulus pada6k.

Reconstruction/history +preflight full parse tetap O(n). Tidak ada column delete/
reorder/resize, ragged mutation, auto-add, free Preview typing atau speedup.
Middle/header-only/1kolom/11→12kolom UI, large menu/keyboard, active Find/Go-to
physical focus dan Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU belum teruji
desktop. [Kontrak dan batas Stage6k](testing-stage6k.md).

## Stage 6j — explicit blank body row before / after

[CI 39865519334](https://github.com/Indrawan007/Hylite/actions/runs/39865519334),
`d5e009b99bff9ee72e1317bd66c2809a033894e3`, **success**, job `120126611644`,
22 Oktober2026 22:30:58–22:38:34 UTC /23 Oktober Makassar. Format, strict Clippy,
native debug build, **649 Cargo**, **235 strict core** (subset), **37 Python**
dan tujuh suite private Xvfb **116+9+9+9+14+16+11 GUI** lulus. Total **184 GUI**
=164 clipboard (155 Source +9 draft),12 title,8 natural exits.

Insert suite membuktikan canonical blank row before/after dengan indent anchor,
exact old bytes, one-step Preview Undo/Redo, Split caret/focus di first blank
cell lewat native X tanpa refocus, distinct typing/insert history, welcome clean
dan natural close0.17 tes Rust baru mencakup forged/cold/stale/nested/caps,
fresh parsed cells/edit sesudah insert, save/branch/modal/file/search semantics.
Semua17 body exercise lama AST-identical; oracle/deadline/refocus utuh. Runner
ketujuh tetap alarm180s/step5min, job45min. Artifact `12463529269`,2.049.263 bytes,
belum expired saat diperiksa; screenshot belum direview visual.

Tidak ada header-only insertion, auto-row/column operations, multi-cell selection,
free Preview typing atau speedup. Large menu/keyboard, active query/Go-to focus,
ragged/1kolom/12kolom/3spasi UI dan Arch/Hyprland/Wayland/IME/RTL/HiDPI/GPU belum
diverifikasi desktop. [Kontrak dan batas Stage6j](testing-stage6j.md).

## Stage 6i — navigasi draft / explicit Apply & Next

[CI 39863457232](https://github.com/Indrawan007/Hylite/actions/runs/39863457232),
`983b517e41407fda69432d54851d127e648e867c`, **success**, watch exit0.
Job `120119664548`: format, native debug build, strict Clippy, **632 Cargo**,
**231 strict core** (subset Cargo), **35 Python**, dan enam suite private Xvfb
**116 regression + 9 batch + 9 alignment + 9 rows + 14 cells + 16 navigation**
lulus. Total **173 GUI** = 154 clipboard (145 Source + 9 draft), 12 title,
7 exit alami. Alarm180s/step5min masing-masing dan job45min tetap. Seluruh16
body exercise lama sama secara AST; tidak ada relaksasi oracle atau refocus.

Annotation suite navigasi membuktikan header/body Previous/Next, dirty refusal
menjaga input, Unicode Apply & Next dengan fresh offset, batas last cell, no-op,
Cancel hanya draft kini, one-step Undo/Redo, Split source focus/caret dan native
clean exit. Artifact `12462558973`, 1.975.135 bytes, belum expired saat diperiksa;
screenshot belum direview visual. Run22 Oktober UTC /23 Oktober2026 Makassar.

Preflight full parse O(n), bukan speedup. Tidak membuka free typing/glyph editing,
multi-cell selection, session undo, row auto-add atau desktop Wayland proof.
Header Apply & Next/EOF UI, query/Go-to physical focus, 16KiB/IME/RTL/HiDPI/GPU
belum terverifikasi desktop. [Kontrak dan bukti Stage6i](testing-stage6i.md).

## Stage 6h — explicit raw cell draft Apply/Cancel

[CI 39792827763](https://github.com/Indrawan007/Hylite/actions/runs/39792827763),
`38595306de76410432bf7b24e5b788282658bf19`, **success**, `gh run watch --exit-status`
keluar0. Job `119873032327`: native debug build, format, strict Clippy,
**611 Cargo**, **226 strict core** (subset Cargo), **33 Python** dan
**116 regression + 9 task-batch + 9 table-alignment + 9 table-rows + 14 table-cells
GUI** lulus. Kelima suite private Xvfb terpisah, alarm180s/step5min masing-masing,
job45min tetap. Total157 checks = 139 clipboard (137 Source + 2 draft input),
12 title, 6 exit alami.

Cells step 22 Oktober2026 14:14:40–14:15:03 UTC membuktikan raw draft input focus,
Unicode paste, Cancel, Apply byte-exact, no-op/invalid refusal, Preview Undo/Redo,
Split Source focus/caret dan natural Close. Semua step termasuk Complete job
success14:15:06, run final success updated14:22:22. Beberapa endpoint job/check-run
sempat tertinggal publikasi status/annotation; tidak dianggap diagnosis startup.
Artifact `12432681330`, 1.866.592 bytes belum expired saat diperiksa; screenshot
belum direview visual. Semua 15 body exercise functions lama tetap sama secara
AST dengan Stage6g; tidak melonggarkan gate/koordinat/refocus/deadline lama.
[Kontrak dan batas Stage6h](testing-stage6h.md).

Ini panel raw Markdown single-line16KiB, root canonical rectangular cells saja,
bukan inline/free text Preview WYSIWYG, pixel/glyph proof atau verifikasi
Arch/Hyprland/Wayland/portal/GPU/IME/RTL. Source tetap satu-satunya dokumen;
unapplied draft tidak pernah ikut Save.

## Stage 6g — whole raw body row tabel root

[CI 39787971305](https://github.com/Indrawan007/Hylite/actions/runs/39787971305),
`3590d56d6e5933d85b7ad74e868b5fc562a97ee0`, job `119856055204` **success** pada
22 Oktober 2026, 13:34:07–13:41:52 UTC. Native debug build, format, strict Clippy,
**581 Cargo**, **218 strict core** (subset Cargo), **31 Python**, **116 regresi +
9 task-batch + 9 table-alignment + 9 table-rows GUI** lulus. Keempat suite GUI
private Xvfb terpisah, alarm180s/step5min masing-masing, job45min tetap. Total143
checks = 126 clipboard + 12 title + 5 exit alami.

Row step 13:41:33–13:41:46 UTC membuktikan raw Duplicate/Delete, scoped history,
Split Source focus/caret tanpa refocus dan natural Close. Header/delimiter/raw
inline/UTF-8/escaped pipe/ragged row/suffix/terminal tab dipertahankan. Cap lama
128 row/512 sel dan snapshot/root provenance diperiksa di planner/model/controller.
Semua 12 body phase regresi dan body batch/alignment tetap sama secara AST dengan
Stage 6f, tanpa perubahan oracle/koordinat/deadline gate lama. Artifact
`12431131246`, 1.742.544 bytes belum expired saat diperiksa; screenshot belum
direview visual. [Kontrak dan batas Stage 6g](testing-stage6g.md).

Bukan WYSIWYG umum, arbitrary cell editing, pixel/glyph proof atau verifikasi
Arch/Hyprland/Wayland/portal/GPU/IME/RTL; EOF tanpa LF/non-pipe row konservatif
belum memperoleh row actions.

## Stage 6f — alignment kolom tabel root

[CI 39784983010](https://github.com/Indrawan007/Hylite/actions/runs/39784983010),
`894227f3d863a396bc2f05a8fccbb5d1a8571fd5`, job `119845680831` **success** pada
22 Oktober 2026, 13:13:43–13:20:38 UTC. Native debug build, format, strict Clippy,
**556 Cargo**, **210 strict core** (subset Cargo), **29 Python**, **116 regresi +
9 task-batch + 9 table-alignment GUI** lulus. Suite GUI tetap tiga private Xvfb
terpisah, alarm 180s dan step 5 menit masing-masing, bukan deadline lebih besar.
134 checks = 118 clipboard + 12 title + 4 exit alami. Artifact `12428428377`
1.668.936 bytes belum expired saat diperiksa; screenshot belum direview visual.

Run pertama 39784221190 lulus native dan 116+9 GUI lama tetapi oracle suite baru
menabrakkan suffix `--:` pada token Center kolom pertama. Actual produk benar di
kolom kedua; oracle dipersempit ke delimiter kedua dengan pipe/spacing dan helper
mengunci literal expected line independen. Tidak mengubah gate regresi lama,
produk, refocus/klik atau deadline. Run success di atas menguji koreksi tersebut.
[Kontrak dan batas Stage 6f](testing-stage6f.md). Ini bukan WYSIWYG umum, pixel/
alignment glyph proof atau verifikasi Arch/Hyprland/Wayland/portal/GPU/IME/RTL.

## Bukti baseline Source yang diamati

[GitHub Actions run 39707517350](https://github.com/Indrawan007/Hylite/actions/runs/39707517350)
**success**, job **Linux build, lint, and tests**, pada commit
`373e05584a070049c52ab515395db1575ce52baf` di branch `arena/21a4c00b-hylite`.
Runner Ubuntu 24.04; toolchain Rust stable melalui workflow, bukan pin versi compiler.

| Pemeriksaan | Hasil |
| --- | --- |
| `cargo fmt --all -- --check` dan format harness | Lulus |
| Harness core dengan `clippy-driver`, `-D warnings` | 150 passed, 0 failed |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Lulus |
| `cargo build --locked --all-targets --all-features` | Lulus, profil debug/dev |
| `cargo test --locked --all-targets --all-features` | 312 passed, 0 failed, 0 ignored |
| Tes Python helper diagnostik CI | 3 passed |

**150 tes core termasuk dalam 312 tes Cargo**, lalu juga dijalankan secara mandiri;
angka tersebut bukan 462 tes Rust unik. Suite Cargo mengompilasi dan menguji kode
aplikasi dengan dependensi asli Iced, parser Markdown, konfigurasi dan filesystem.
Tes filesystem memakai direktori sementara; tes controller tidak menjalankan task
dialog/window yang dikembalikan. Tidak ada jendela atau sesi Wayland yang diuji **dalam run headless awal ini**.

Update runtime berikutnya: [CI run 39710774126](https://github.com/Indrawan007/Hylite/actions/runs/39710774126)
pada `ee73f2b` juga lulus, termasuk **smoke GUI aplikasi sebenarnya** melalui
X11/Xvfb + tiny-skia dan 9 tes Python helper. Lihat [bukti GUI terpisah](testing-gui-x11.md);
pengujian desktop Wayland/Arch/Hyprland tetap belum dilakukan.

Update navigasi/fokus: [CI run 39711916704](https://github.com/Indrawan007/Hylite/actions/runs/39711916704),
commit `72835cb`, lulus **315 tes Cargo**, 11 tes Python helper dan 34 pemeriksaan
GUI X11 (23 di antaranya tanpa refocus mouse). Tiga tes Cargo tambahan menguji
operasi fokus Source: idempoten, scope tidak menyentuh widget lain, dan scope
hilang adalah no-op. Smoke tanpa klik menemukan kehilangan fokus setelah undo;
operasi `focus_next` diganti dengan fokus idempoten dalam subtree container
Source. Lihat detail dan batas buktinya di [catatan GUI](testing-gui-x11.md).

Update lifecycle: [CI run 39713905090](https://github.com/Indrawan007/Hylite/actions/runs/39713905090),
commit `d03197b`, lulus **320 tes Cargo**, **14 tes Python helper**, serta **63
pemeriksaan GUI/lifecycle**. Lima regresi Rust tambahan mencakup shortcut
konfirmasi, title pending, no-op tanpa prompt, konteks Cancel dan pencegahan
Alt accelerator jatuh ke penyisipan karakter. Native close dirty + Discard dan
clean close benar-benar menghasilkan exit 0 sebelum fallback cleanup, bukan
SIGTERM. Lihat [bukti dan batas lifecycle](testing-lifecycle-x11.md). Portal file
serta close ketika save in-flight tetap belum diuji end-to-end.

Update renderer Stage 5a: [CI run 39716266646](https://github.com/Indrawan007/Hylite/actions/runs/39716266646),
commit `ea182b1`, lulus **347 tes Cargo**, **15 tes helper Python**, **67
pemeriksaan GUI X11**, build debug/format/Clippy strict. Tambahan 27 tes Rust
menguji model owned block/inline, adapter Iced dan invariant Source saat view
berubah; 4 pemeriksaan clipboard baru menguji Preview read-only dan round-trip
Source/Preview/Split tanpa perubahan/undo tambahan. 150 core adalah subset 347.
Screenshot bukan pixel assertion; model range belum menjadi source↔caret map.
Lihat [detail, artifact dan batas Stage 5a](testing-stage5a.md).

Update tabel Stage 5b: [CI run 39717665832](https://github.com/Indrawan007/Hylite/actions/runs/39717665832),
commit `414c0f0`, lulus **361 tes Cargo**, **16 helper Python**, **71 pemeriksaan
GUI X11**, build debug/format/Clippy strict. Tambahan 14 tes Rust terdiri dari 12
tes model tabel dan 2 tes adapter; satu tes fallback lama diganti menjadi tes
struktur tabel. Controller/view invariant diperluas. Empat clipboard check baru
memeriksa Source byte-exact sebelum/sesudah mode cycle, input/wheel saat Preview,
dan satu Undo kembali ke baseline. 150 core tetap subset 361; scrollbar/glyph/
layout visual bukan assertion otomatis. Lihat [bukti dan batas tabel](testing-stage5b.md).

Update cache/mode fokus Stage 5c: [CI run 39719585330](https://github.com/Indrawan007/Hylite/actions/runs/39719585330),
commit `b614f98`, lulus **374 tes Cargo**, **17 helper Python**, **76 pemeriksaan
GUI X11**, format/build debug/Clippy strict. Tambahan 13 tes Rust menguji cache
lazy, invalidasi/reuse, span borrowed dan invariant/routing view. Lima clipboard
check baru melatih warm Split, Go to Line Unicode ke sel tabel, edit/undo/redo
serta Source clean kembali tanpa klik. 150 core adalah subset 374.
Percobaan awal lulus 373 Rust tetapi menemukan fokus hilang saat Split → Source;
controller sekarang memakai task scoped restore, bukan melemahkan probe.
Lihat [detail dan batas cache](testing-stage5c.md); ini bukan benchmark speedup,
layout/pixel oracle atau source↔selection mapping.

Update provenance navigation Stage 5d: [CI run 39728608200](https://github.com/Indrawan007/Hylite/actions/runs/39728608200),
commit `40771f1`, lulus **396 tes Cargo**, **158 core mandiri** (subset Cargo),
**18 helper Python**, **80 pemeriksaan GUI X11**, format/build debug/Clippy
strict. Tambahan 22 tes Rust meliputi resolver snapshot+UTF-8, membership model,
stale/edit/replace/metadata, guard/query/history controller dan action renderer.
Empat clipboard check baru membuktikan klik tombol provenance membuka Split,
keyboard sentinel pada awal first-heading dan dua Undo mempertahankan source
Unicode/terminal tab serta baseline clean. Tidak ada klik tambahan ke Source.
Ini block-start anchor, bukan glyph/caret hit-testing, selection mapping atau
scroll sync. Detail, artifact dan checklist manual: [Stage 5d](testing-stage5d.md).

Update cell provenance Stage 5e: [CI run 39731339080](https://github.com/Indrawan007/Hylite/actions/runs/39731339080),
commit `0a531da`, lulus **411 Cargo**, **162 core mandiri** (subset Cargo),
**19 Python**, **84 GUI X11**, format/build debug/Clippy strict. Lima belas tes
Rust baru menguji scope parent-table/row/column/range, UTF-8/stale/cold/padded,
cache/metadata/redo, adapter dan guard controller. Empat clipboard check baru
membuktikan klik kiri read-only serta klik kanan Count → raw cell start sebelum
`**3**`, bukan glyph hasil styling, setelah prefix Unicode, tanpa klik Source.
Dua Undo mengembalikan fixture dan welcome clean. CI awal `39731115281` berhenti
saat lint: helper tes mengakses modul privat; helper sekarang menggunakan offset
literal fixture tanpa memperluas visibility atau melemahkan gate. Detail dan
batas bukti ada di [Stage 5e](testing-stage5e.md); belum edit/glyph/selection map.

Update task prototype Stage 6a: [CI run 39733641430](https://github.com/Indrawan007/Hylite/actions/runs/39733641430),
commit `c774f2a`, lulus **439 Cargo**, **172 core mandiri** (subset Cargo),
**20 Python**, **92 GUI X11**, format/build debug/Clippy strict. Tambahan 28 Rust
menguji canonical marker/source provenance, item membership/stale/cold/code,
Atomic history/cache/metadata, adapter dan controller guards/query/save race.
Delapan clipboard check baru mengklik checkbox Preview dan Split, memeriksa
one-byte Source edit serta Undo/Redo byte-exact Unicode/X lain/code/trailing tab,
tanpa refocus Source. Undo masih Source/Split, bukan editing umum Preview.
Percobaan `39733392375` lulus lint/build tetapi fixture model salah menempatkan
baris task penginterupsi tabel sebagai sel; fixture diperbaiki dengan leading
pipe, assert node Table, dan assert kasus interupsi sebagai task nyata. Tidak
mengubah parser atau melemahkan gate. [Kontrak dan batas Stage 6a](testing-stage6a.md).

Update history task Stage 6b: [CI run 39737235985](https://github.com/Indrawan007/Hylite/actions/runs/39737235985),
commit `0df378803bbe19672abb9391ae868306a82a4053`, **success**. Job
`119685695986`, 2026-10-22 06:57:38–07:01:20 UTC: **461 Cargo**, **179 core
mandiri** (subset Cargo), **22 Python**, **101 GUI X11** (87 clipboard, 12 title,
2 exit alami 0), format/build debug/Clippy strict. Stage 6b menambah 22 Rust dan
9 clipboard checks dengan Copy tanpa refocus. Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y
serta toolbar kini boleh mengembalikan task edge teratas di Preview, tetapi
berhenti sebelum transaksi Source; Source/Split tetap history umum. Status
artifact API: `12407691827`, `gui-x11-smoke`, 1.188.202 bytes, belum expired saat
diperiksa. Screenshot belum direview visual; bukan pixel oracle.

Percobaan awal `39736591093` pada `4fed30f` lulus 461 Cargo dan probe history baru,
namun proses kedua lifecycle startup exit 101. **Akar exit 101 belum diketahui.**
Tidak dianggap selesai diperbaiki hanya karena run berikutnya hijau. Annotation
lama 6000 karakter terpotong menjadi 4096 pada Checks API, menyembunyikan bagian
log startup. Reporter sekarang membagi error menjadi 3000 karakter per annotation
(maksimal 8, total tail tetap di bawah 24.000 karakter); regresi mengunci panic
di offset 4500 tetap utuh. Ini perbaikan observabilitas, bukan retry aplikasi,
kenaikan timeout, refocus workaround, atau perubahan expected source. Run baru
di atas lulus semua lifecycle termasuk startup kedua. Pantau jika failure yang
sama berulang; belum ada diagnosis Winit/X11/resource yang diklaim.
[Kontrak dan batas Stage 6b](testing-stage6b.md).

Update heading prototype Stage 6c:
[CI run 39739554830](https://github.com/Indrawan007/Hylite/actions/runs/39739554830),
commit `8e641dde74255eddcd2340979c4857cc4ce29a5b`, **success**. Job
`119693199908`, 2026-10-22 07:18:16–07:23:05 UTC: **486 Cargo**, **187 strict core**
(subset Cargo), **23 Python**, **109 GUI X11** (95 clipboard, 12 title, 2 exit
alami 0), format/build debug/Clippy strict. Tambahan 25 Rust: planner7/history1/
model2/Document8/adapter2/controller5. GUI menambah 8 clipboard check: Preview
H−/H+, Undo/Redo, raw byte preservation, Split Source focus pada marker end dan
transaksi typing/heading terpisah; tidak ada refocus click Source. Artifact API
`12408084904`, `gui-x11-smoke`, 1.267.337 bytes, belum expired saat diperiksa;
screenshot belum direview visual. Cakupan Preview history sekarang checkbox dan
heading-level edge tervalidasi di puncak, bukan history umum. Tidak mengubah
lockfile/dependensi/grammar dan options parser/file worker, timeout 180 detik
atau gate lama.
[Kontrak dan batas Stage 6c](testing-stage6c.md).

Update heading Setext Stage 6d:
[CI run 39765116882](https://github.com/Indrawan007/Hylite/actions/runs/39765116882),
commit `cac30e8797712746c3ca82d60c7fe6d0a56f3445`, **success**. Job
`119778526781`, 2026-10-22 10:42:33–10:46:34 UTC: **510 Cargo**, **194 strict core**
(subset Cargo), **24 Python**, **116 GUI X11** (102 clipboard, 12 title, 2 exit
alami 0), format/build debug/Clippy strict. Tambahan 24 Rust: planner7/model2/
Document8/adapter2/controller5. GUI menambah 7 clipboard checks: H+/Undo/Redo
Preview dan H− Split mempertahankan multiline raw body/indent/panjang underline/
whitespace; sentinel Source tepat di underline end tanpa refocus click. Atomic
Setext dan native typing harus menjadi dua Undo, lalu fixture Undo ke welcome
clean. Artifact API `12419393417`, `gui-x11-smoke`, 1.367.061 bytes, belum expired
saat diperiksa; screenshot belum direview visual. Kontrak ATX lama tetap ditolak
untuk Setext; action/metadata terpisah memperluas heading-level edits tanpa
konversi format. Tidak mengubah grammar/options parser, dependensi/lockfile,
serializer, worker file, budget history/timeout/lifecycle gate atau menambah
hook/retry. [Kontrak dan batas Stage 6d](testing-stage6d.md).

Update batch task Stage 6e:
[CI run 39771530257](https://github.com/Indrawan007/Hylite/actions/runs/39771530257),
commit `3fb4199beffc62afca5fb7c5f40899bc2cbbb0aa`, **success**. Job
`119799991102`, 2026-10-22 11:33:56–11:38:05 UTC: **530 Cargo**, **201 strict core**
(subset Cargo), **27 Python**, **116 regression + 9 task-batch GUI** (dua suite:
total 110 clipboard, 12 title, 3 exit alami 0), format/build debug/Clippy strict.
20 Rust baru: planner7/Document8/controller5. Batch Check/Clear menjaga source
bytes/kapital X/unsupported/literal, one-step Preview Undo, no-op dan Source
focus Split; complete-list warm membership dan cap 1024 menolak subset/forged/
stale/cold/oversized tanpa prefix parsial. Tidak membuka general Preview input.

Regresi default tetap semua 116 gates/180s; suite batch memakai invokasi Xvfb
privat terpisah dengan XDG/app/clipboard baru dan alarm 180s sendiri. Workflow
menambah step 5 menit, tidak menaikkan deadline regresi atau mengurangi pemeriksaan
lama. Regresi 11:35:08–11:37:48 UTC; batch 11:37:48–11:38:01 UTC. Artifact API
`12422742801`, `gui-x11-smoke`, 1.520.409 bytes, belum expired saat diperiksa;
root evidence lama + `task-batch/` baru, screenshot belum direview visual.

CI awal `39770915857` lulus Rust tetapi gagal reveal source: label batch 16 px
pada width 112 dan default horizontal padding 20 px dapat wrap dan menambah tinggi
toolbar. Fix produk `3fb4199` memakai label 14 px; seluruh probe lama kembali
lulus dengan koordinat/bytes/refocus/deadline yang sama. Ini bukan mengabaikan
failure atau menambah retry runtime. Startup kedua 6b exit 101 lama masih belum
diagnosis, tidak diklaim diperbaiki oleh fitur atau pemisahan suite ini.
[Kontrak dan batas Stage 6e](testing-stage6e.md).

## Perbaikan yang dipicu CI

1. Clippy menolak task Iced `must_use` yang diabaikan tes controller. Tes sekarang
   menulis `let _ = ...` secara eksplisit; tidak ada penekanan lint dan task dialog
   tetap tidak dieksekusi oleh tes simulasi controller.
2. Setelah lint/build lulus, satu dari 309 tes gagal: undo empty-item Enter tidak
   memulihkan tab akhir. Akar masalahnya adalah `cosmic-text 0.12.1` `insert_at`
   yang membuang suffix `char::is_control` di segmen Paste, termasuk tab akhir,
   bukan hanya delimiter LF. Ekspektasi tes lama **tidak dilonggarkan**.
3. `document/state.rs::paste_exact` melindungi Paste dengan satu spasi sementara,
   lalu native Backspace menghapus hanya spasi itu. Load, rekonstruksi caret
   suffix/prefix, dan paste pengguna memakai helper yang sama. Operasi internal
   tidak membuat entri history/revisi tambahan; paste tetap satu transaksi.
4. Tiga regresi native ditambahkan: load trailing tab/LF, caret pada setiap split
   UTF-8 fixture bertab, dan paste pengganti seleksi dengan undo/redo atomik.
   Total suite menjadi 312; semuanya lulus, termasuk tes lama yang menemukan bug.
5. `scripts/ci-cargo.sh` mempertahankan exit status Cargo dan melaporkan diagnostik
   terbatas lewat GitHub Checks. Ini memungkinkan pembacaan hasil ketika unduhan
   arsip log Actions tidak dapat diakses dari lingkungan penyuntingan. Helper
   reporter diuji untuk escaping, command suspension, batas output, dan exit status.

## Batas verifikasi dan langkah berikutnya

- Ini build debug dan tes headless Ubuntu, **bukan** kelulusan desktop Arch Linux +
  Hyprland, bukan build release/AUR/packaging, dan bukan jaminan kesiapan produksi.
- Callback hasil file/controller, pemetaan binding dan model native diuji langsung;
  dialog portal, D-Bus, keyboard compositor dan save end-to-end dari
  jendela nyata belum diuji. Native client close X11 kini diuji pada suite lifecycle,
  bukan WM/compositor close atau close saat save berjalan. Tes file sementara bukan simulasi power-loss/recovery.
- Fokus, selection/IME/RTL, shaping/soft-wrap, syntax rendering, scrolling/caret
  viewport dan shortcut yang mungkin diambil Hyprland masih membutuhkan desktop.
- Batas algoritme Source, history snapshot O(n), LF canonical, selection collapse,
  gutter yang ditunda, dan fitur Markdown yang belum lengkap tetap berlaku.
- Jalankan [checklist Tahap 2](testing-stage2.md), [Tahap 3](testing-stage3.md),
  dan [Tahap 4](testing-stage4.md) pada salinan dokumen dan konfigurasi sementara.
  Jangan mencentang skenario desktop hanya karena suite Cargo lulus.

Gate build/lint/tes native baseline terpenuhi. **Tahap 4 belum dinyatakan selesai.**
Stage 5a–5e renderer/navigasi serta 6a checkbox/6b history/6c ATX/6d Setext/
6e batch task, 6f alignment kolom, 6g raw body row dan 6h panel draft teks sel serta 6i navigasi/Apply & Next dan 6j insert baris serta 6k insert/6l hapus kolom dan 6m pindah body row/6n pindah kolom root terbatas tersedia; hasil dan batas terbaru ada di [catatan Stage 6n](testing-stage6n.md).
WYSIWYG umum tetap belum ada.
Tidak ada merge ke `main` dalam verifikasi ini.
