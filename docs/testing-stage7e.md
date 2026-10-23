# Stage 7e — atomic multiline Apply & Next

Melanjutkan [navigasi7d](testing-stage7d.md). Tombol **Apply & Next** menerapkan satu
paragraf multiline root lalu membuka eligible multiline root berikutnya. Previous/
Next unchanged, Apply-and-close, Enter newline, Escape Cancel dan local Undo7c tetap.
Tidak menambahkan Apply & Previous, shortcut Apply, block creation atau inline WYSIWYG.

## Kontrak

- Target awal harus exact **warm current root**7b. Eligible tetap2–128baris nonempty/
  unindented, payload≤16KiB, LF final, no controls selain LF internal. Next mengikuti
  urutan Source7d, melewati single-line/heading/nested/code/literal/unsupported,
  tanpa wrap atau implicit creation.
- **Boundary last menolak sebelum Apply**, changed maupun unchanged. Tombol disabled
  bila next tidak ada/error; direct message mempertahankan draft/history/Source dengan
  notice. Apply biasa tetap tersedia untuk menyelesaikan paragraf terakhir.
- Changed Apply & Next wajib mem-preflight **full candidate Source**: edited range
  tetap complete root multiline Paragraph; old next root bergeser tepat menurut delta
  byte UTF8, tetap complete eligible root dan payload next persis sama. Split/conversion/
  absorption/neighbor loss ditolak tanpa partial Apply. Byte prefix/final LF/suffix/
  skipped neighbors tetap raw; tidak AST serialization atau newline normalization.
- Two-phase opaque plan: prepare read-only +candidate parse +**native Source content/
  applied caret reconstruction**. Controller membangun dan memverifikasi **native next
  Draft** sebelum commit; gagal/abandoned plan tidak mengubah Source/history/cache.
  Plan private fields menyimpan full pre-edit Source, anchor snapshot/ranges dan
  preflight model/buffer. Provisional next target belum live. Commit re-auth exact
  warm anchor dan full Source equality; stale/cold/replaced atau kandidat dengan Source
  lain ditolak meski anchor text sama. Caret-only change/Save ack boleh selama
  identity/text tetap cocok. Plan sekali pakai, tidak disimpan asinkron.
- Setelah semua preflight, satu native Source swap +shared `record_edit` sebagai
  **PreviewParagraph atomic edge**, shared identity/history caps/barriers unchanged.
  Model candidate dipasang warm; target next distamp snapshot final, bukan revision
  yang diprediksi. Tidak ada fallible parse/native initialization setelah commit.
- Sesi next baru: raw payload exact, native caret0, local Undo/Redo/error/notice kosong,
  neighbor flags current. Previous dari next mengarah ke raw first yang sudah diedit.
  Source caret tetap **akhir payload yang diterapkan**, bukan lokasi draft next.
  Cancel next tidak meng-Undo Apply sebelumnya; Source/Split focus/caret kembali ke
  prior applied payload. Tiap effective Apply berbeda satu Source edge.
- Unchanged Apply & Next hanya pindah: no revision/history/cache/checkpoint/redo/caret/
  search refresh. Membuka sesi lokal baru, redo draft lama dibuang seperti7d. Source
  model tidak di-parse ulang atau diganti. Current draft changed boleh Apply & Next;
  notice dirty Previous/Next tidak menghalangi Apply & Next yang valid.
- Invalid/structural/stale/native-init refusal mempertahankan accepted raw draft,
  selection/local history; scope focus restored. Restoring local Undo setelah parser
  refusal tidak bypass auth/full parser pada percobaan berikutnya. Source Undo/Redo/
  PreviewHistory tetap blocked selama modal; local empty Ctrl+Z tidak undo Source.
- Existing mode/prompt/Open/other-modal guards, immutable Save/Export tokens/replies,
  checkpoint identity, logical Find/Go-to dan Source barriers tetap. Search refresh
  hanya effective commit. Single-line/cell panels dan Source3helpers unchanged.
- New160×32button append setelah Previous/Next di baris kedua yang sama. Posisi/height
  semua old controls/root bars/footer unchanged. No new shortcut/refocus hack.
  Disabled button click bisa blur; Copy-before-click→Cancel bukan disabled-focus proof.

## Implementasi / tes

`Document::prepare_multiline_paragraph_next` menghasilkan opaque `MultilineNextPlan`
dengan getter provisional target/value/neighbor flags. Private prepared edit owns
verified Content/text/full blocks; controller verifies Draft before consuming plan.
`commit_multiline_paragraph_next` checks current context again then shared native swap/
history recorder installs model and stamps final snapshot. Existing Apply/Move paths
unchanged. Full source copies/parse/native rebuild O(n), peak extra buffers/model;
bukan incremental/performance/RAM improvement claim.1MiB cap berlaku retained draft
history text, bukan two-phase full-Source memory/process RAM.

**886 Cargo** =869previous +**17Rust** (**8Document +7controller +2UI**). Local
**294strict core** unchanged subset, **59Python** =57+2.

- Document: exact raw next/UTF8 growth/fresh snapshot/model/caret/atomic Undo/Redo,
  unchanged retains cache pointer/caret/checkpoint/Source redo, last changed/noop,
  lexical/control/structural failures, cold/eight forged/single/nested/literal/stale,
  abandoned/native prefill before commit, cold/stale plan, foreign Document same
  anchor/different suffix, sequential commits and Source typing barrier, Save ack/
  caret interleave but replace rejected, byte/line capacity/grow/shrink next offsets.
- Controller: Preview/Split fresh next/local stack/caret/Source edge, local Undo→noop
  transition preserves Source redo, boundary notice keeps explicit Apply available,
  lexical/parser/oversized/stale refusal/local history retained and correction,
  Source/pending/Open/no-draft/other-modal guards, backward fresh applied payload,
  Source barrier, immutable Save/Export replies +logical search/Go-to refresh.
- UI: valid dirty/notice Apply Next enabled; next bound/error/invalid disabled;
  native exact next buffer at caret0/fresh history constructed before Source change.
  All prior tests retained, no grammar/history/portable count changes.

## Private GUI suite18

`--suite paragraph-apply-next`: private app/Xvfb/XDG/env/clipboard,180s runner alarm,
5min separate step/45min job unchanged. **28old exercise bodies AST-identical against
`bb37223`**; all17old gates unchanged, no byte/coordinate/oracle/deadline/refocus
weakening, no retry/runtime hooks.

**23checks** =22Copy(11Source+11draft)+natural clean Close0:

1. Raw first opens focused; Unicode3line staged, explicit Apply & Next replaces only
   first and loads exact second with native focus/fresh offsets. Empty local Ctrl+Z
   keeps next/Source commit. Previous shows grown raw first; unchanged Apply & Next
   advances without Source edge. Changed last draft/disabled boundary→Cancel leaves
   only initial Apply. One Preview Undo/Redo restores original/applied Source.
2. Split lexical-valid `one\n# heading` stages, Apply Next parser refusal preserves
   raw draft/focus, Cancel keeps previously committed Source, no partial first line.
3. Split final first Apply Next opens raw second; Cancel then native X without Source
   refocus proves caret at prior applied **first** payload end (not second). Typing/
   final Apply/initial Apply/paste Undo separately return exact previous/original/
   welcome clean; natural client Close0. No hidden noop/boundary/refusal/Cancel edges.

Nine product Open/Previous/ApplyNext clicks including disabled last boundary. No
refocus click; native Copy proves raw value/focus, not selection anchor/direction/
pixel glyph mapping. `paragraph-apply-next/paragraph-apply-next-refused.png`,
`paragraph-apply-next-split.png` not pixel oracles. Aggregate **352GUI**
=321clipboard(263Source+58draft),12titles,19natural exits.

```sh
./scripts/test-gui-x11.sh --suite paragraph-apply-next --artifacts "$(mktemp -d)"
# Keep all17old suites too, never substitute.
```

## Execution status

Local fmt/harnessfmt/294strict core/59Python/diff/AST28 check PASS.

[CI 39888202649](https://github.com/Indrawan007/Hylite/actions/runs/39888202649)
(`64cc13f`) **gagal** di test-only `unused_mut` tes baru, sebelum build/Clippy/GUI.
Perbaikan `8ba6602` hanya menghapus `mut` fixture read-only; tidak melonggarkan
Clippy dan tidak mengubah produk/bytes/koordinat/oracle/deadline/refocus.

[CI 39888436003](https://github.com/Indrawan007/Hylite/actions/runs/39888436003),
`8ba6602ca1dcb19f8522332b8b4eb8f07096f0ef`, job120202070235 **SUCCESS**,
2026-10-23 **02:13:55–02:25:14 UTC** / **10:13:55–10:25:14 Makassar**. Check
formatting, strict core subset, Clippy, build, **886Cargo**, seluruh **18suite**
(**352GUI**,23suite baru) dan upload artifact lulus satu kali. Artifact
`12474580471`,**3.158.202bytes**, SHA256
`4d02ac3bb9d5e751d93f0f448488d2ccca9e8a348461f6a9af540a8d8bc3f214`, expires
2026-10-30T02:25:09Z, unexpired. Native counts dari check annotations; screenshot
tidak direview visual dan full archive tidak diperiksa.

## Manual / limits

- [ ]Multiple/large/offscreen roots and transitions at128lines/16KiB, native failure
  UI, selection direction, disabled click blur/keyboard/narrow window/accessibility,
  physical query/Go-to focus and scroll/layout/visual glyph behavior.
- [ ]Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU. X11 tiny-skia isn't target
  compositor proof; screenshots not visually reviewed.

No Apply & Previous/cross-kind navigation/EOF/blank/indented editing, implicit Apply/
create/delete/reorder, local history single-line/cell, rich selection/inline WYSIWYG/
glyph mapping/multi-block selection/sync scroll/incremental parse/performance claim.
Source remains authoritative; full candidate preflight, bounded Source history and
immutable file pipeline preserved. Future source transformations must not reuse
candidate targets as live provenance before final commit stamping.
