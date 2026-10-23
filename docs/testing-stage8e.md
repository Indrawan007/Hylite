# Stage 8e — math read-only subset

**Hasil:** selesai pada branch `arena/21a4c00b-hylite`, kode `97e586c`.
CI [39963797627](https://github.com/Indrawan007/Hylite/actions/runs/39963797627)
lulus (job `120448270375`, 23 Oktober 2026 13:45:06–13:57:38 UTC /
21:45:06–21:57:38 Makassar). Tidak menambah crate.

## Kontrak yang diterapkan

`Options::ENABLE_MATH` yang sudah aktif kini punya proyeksi native untuk event
`InlineMath` (`$…$`) dan `DisplayMath` (`$$…$$`). Formula sukses menjadi satu run
Unicode read-only. Math display yang berdiri sendiri menjadi block berpanel;
formula inline tetap berada di baris paragraf, dan formula di sel tabel juga
memakai subset yang sama.

Subset dependency-free di `src/math.rs`:

| Struktur | Bentuk tampilan sederhana |
|---|---|
| `\frac{a}{b}` | `a⁄b`; bagian majemuk diberi tanda kurung |
| `\sqrt{x}` | `√x`; bagian majemuk diberi tanda kurung |
| `^` / `_` | Superskrip/subskrip Unicode untuk karakter yang tersedia; selain itu `^(…)` / `_(…)` |
| `\left(…\right)` | Delimiter berpasangan yang dideklarasikan (termasuk `.`, `|`, `\langle`, `\vert`) |
| Greek | `alpha`, `beta`, `gamma`, `delta`, `epsilon`/`varepsilon`, `zeta`, `eta`, `theta`/`vartheta`, `iota`, `kappa`, `lambda`, `mu`, `nu`, `xi`, `pi`, `rho`, `sigma`, `tau`, `upsilon`, `phi`/`varphi`, `chi`, `psi`, `omega`; kapital `Gamma`, `Delta`, `Theta`, `Lambda`, `Xi`, `Pi`, `Sigma`, `Phi`, `Psi`, `Omega` |
| Operator/fungsi | `times`, `cdot`, `pm`, `mp`, `div`; `le`/`leq`, `ge`/`geq`, `ne`/`neq`, `approx`, `equiv`; `to`, `rightarrow`, `leftarrow`, `Rightarrow`, `Leftrightarrow`; `infty`, `sum`, `prod`, `int`, `partial`, `in`, `notin`, `subset`/`subseteq`, `supset`/`supseteq`, `cup`, `cap`, `forall`, `exists`, `nabla`, `angle`, `degree`, `prime`, `ldots`/`dots`, `cdots`, `log`, `ln`, `sin`, `cos`, `tan`, `lim`, `max`, `min` |

Hasil adalah **linearization Unicode satu dimensi**, bukan formula dua dimensi.
Whitespace tepi dipangkas; parsing dibatasi 2.048 byte per rumus dan 32 tingkat
group. Bila ada macro yang tidak dikenal, group/script tidak seimbang, delimiter
salah, atau batas terlampaui, seluruh rumus ditolak: Preview menampilkan
`⟦math source fallback: $…$ — alasan⟧` (atau bentuk `$$…$$`) sebagai teks
beraksen/monospace, bukan mencampur bagian hasil parse dengan source yang gagal.

Raw delimited source juga disimpan dalam `Run.math`. Ini bukan peta glyph dan
tidak dapat dipakai untuk mengedit Source. HTML, footnote, link, dan blok lain
tetap mengikuti kontrak lamanya. PDF Stage 8d sengaja **tetap menulis source math
secara inert** (monospace); hanya Preview yang mendapat subset ini.

## Gate Rust dan Python

| Gate | Cakupan Stage 8e |
|---|---|
| `src/math.rs` | 9 unit test: simbol/operator; pecahan/akar; sup/sub dan urutan; delimiter/escape; command tak dikenal; malformed/nesting; batas byte; proyeksi sukses/fallback; jalur refusal tanpa panic |
| `preview_model_tests.rs` | Inline sukses, unsupported ber-marker dan source delimited, display block, serta inline math supported/unsupported dalam sel tabel sambil menjaga HTML/gambar/footnote tetap inert |
| `preview.rs` | Widget native dibangun untuk math inline/display/fallback dalam tema terang dan gelap; ini bukan oracle piksel |
| PDF mapping | Regresi yang sudah ada memastikan PDF masih menerima source math, bukan linearization Preview |
| Python | 3 helper test untuk fixture byte-exact, pemeriksaan suite, dan dispatch eksklusif; semua 88 helper test lulus |

`math.rs` juga masuk harness dependency-free `scripts/test-source-core.sh`:
**335 strict core** lulus (`RUSTC=/home/user/.cache/clippy.sh`).

## Suite GUI X11: ke-28 `math`

`MATH_DOCUMENT` adalah fixture ASCII byte-exact dengan rumus inline, display,
dan `\begin{matrix}` yang sengaja di luar subset. Suite meluncurkan aplikasi
pada dokumen itu, menyalin Source secara persis, menangkap screenshot Source /
Preview / Split, mencoba paste ketika Preview aktif, kembali ke Source, lalu
memverifikasi bytes tetap identik dan window keluar natural.

**7 pemeriksaan:** title dokumen; 2 salinan Source byte-exact; Preview inline /
display / fallback; Split; Source tetap bersih setelah paste; native exit.
CI keseluruhan: **1.026 Cargo / 335 strict core / 88 Python / 28 suite GUI, 505
pemeriksaan** (446 clipboard, 18 title, 28 exit native). Artefak `12507250289`
(`gui-x11-smoke`, 4.172.220 byte, kedaluwarsa 2026-10-30T13:57:35Z).

Suite ini membuktikan jalur aplikasi hidup, mode read-only, kestabilan Source,
dan clean close; screenshot hanyalah evidence. Tidak ada oracle piksel / aksesibilitas
atau pembaca equation, jadi **tidak** membuktikan glyph/spacing visual, KaTeX parity,
font math tersedia di semua mesin, maupun perilaku Wayland/Hyprland.

## Batas eksplisit

Tidak ada KaTeX/MathJax, rendering dua dimensi, `\text`, matrix/alignment, macro
bebas, accent, nth-root, pemecahan ekspresi di antara karakter yang diketahui dan
tidak diketahui, mapping glyph→byte, input/edit math di Preview, atau klaim
performa. Sintaks di luar daftar tetap source yang diberi tanda; parser tidak
menghapus, menulis ulang, atau mengeksekusi formula. Lihat juga
[arsitektur Stage 8e](architecture.md#math-read-only-subset-stage-8e) dan
[hasil GUI X11](testing-gui-x11.md#stage-8e-suite-ke-28-math-subset-read-only-tanpa-oracle-piksel).
