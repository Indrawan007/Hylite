# Stage 7d — navigasi draft multiline yang unchanged

Melanjutkan [draft lokal7c](testing-stage7c.md): tombol **Previous multiline / Next
multiline** menelusuri paragraf multiline root yang didukung. Bukan glyph mapping,
block conversion, implicit Apply, atau navigasi seluruh jenis blok.
Historical7d limits: [7e](testing-stage7e.md) menambahkan tombol Apply & Next
terpisah; Previous/Next unchanged-only dan Apply-and-close tetap sama.

## Kontrak

- Pilih **eligible multiline root terdekat dalam urutan Source**, melewati heading,
  paragraf satu baris, nested quote/list, code/fallback/literal, EOF tanpa LF dan
  paragraf nonkanonis/oversized. Eligible tetap7b:2–128baris nonempty/unindented,
  payload≤16KiB, LF final, no control selain LF internal. Tidak melonggarkan7a/7b.
- Navigasi hanya ketika **raw draft sama persis dengan payload Source saat ini**
  dan tidak ada error. Draft changed/invalid/parser-refused tetap utuh, termasuk
  native selection dan local history; notice terpisah tidak men-disable Apply
  yang valid. Controller memulihkan named draft input focus setelah refusal.
  Undo draft ke teks asli boleh diikuti navigasi, tanpa auto-Apply/discard teks changed.
- Berhasil pindah = **sesi draft baru**: native caret DocumentStart, local Undo/Redo
  kosong, notice/error kosong, raw payload utuh. Seluruh local history/redo sesi
  sebelumnya dibuang secara sengaja, bahkan bila Undo telah kembali ke teks asli.
  Source caret tidak mengikuti draft/navigation.
- Tidak wrap atau create paragraph. Tombol boundary/error disabled; direct message
  boundary tetap mempertahankan draft dengan notice. Disabled native button click
  dapat blur editor: tidak mengklaim refocus untuk klik yang tidak mengirim message.
  GUI Copy-before-click→Cancel adalah product workflow, bukan refocus workaround.
- Tanpa shortcut baru atau Apply & Next. **Apply eksplisit tetap close** dan satu
  PreviewParagraph atomic edge; Cancel/Escape hanya membuang sesi saat ini. Setelah
  Apply, reopen dari Preview untuk lanjut. No-op Apply tetap tanpa Source edge.
- Semua kandidat/arah berasal dari **warm current root model**; snapshot/block/
  payload target awal harus cocok tepat. Cold/stale/forged/nested/single-line/
  literal tidak di-rebase atau menyebabkan parse/warm. Target neighbor membawa
  snapshot/range current; fresh native Content diverifikasi sebelum sesi lama diganti.
- Navigation sendiri tidak mengubah Source bytes, snapshot/revision/checkpoint/
  dirty, cache pointer/generation, caret, Source Undo/Redo, logical search/Go-to,
  atau inflight Save/Export. Tidak memanggil `edited`/commit/Undo Source. Pending
  confirmation/Open/Source mode/other modal/no-draft guard tetap. Async Save/Export
  replies mengikuti aturan lama, tidak mengganti draft yang sedang dipilih.
- Dua tombol160×32 di **baris baru setelah Apply/Cancel/Undo/Redo lama**. Posisi
  input dan semua tombol lama tetap; footer/notice turun44px. Tidak ada perubahan
  provenance bar, menu prefix, renderer, parser grammar atau Source editor.
- Local snapshots tetap100record total/1MiB text/record16KiB, caret collapsed saat
  local history restore, tanpa selection anchor/scroll restore. Notice dihapus
  hanya oleh effective native edit/local history restore atau sesi baru.

## Implementasi dan tes

`document::ParagraphDirection` dan read-only `adjacent_multiline_paragraph` memilih
root metadata existing setelah authenticate target via API7b. Tidak rekursif ke
children, tidak parse, tidak AST serialization. Controller membangun Draft+neighbor
availability sebelum swap; UI availability hanya affordance, bukan autentikasi.
Source3helper exact-paste/serialization/caret, history7c, single-line dan cell APIs
unchanged. Lookup source-order/root walk serta snapshot text O(n), bukan speedup.

**869 Cargo PASS** =852 sebelumnya +**17 Rust** (**8 Document +7 controller
+2 UI**). **294 strict core** unchanged (subset Cargo), **57 Python** =55+2.
Native Cargo/build/Clippy/all17GUI PASS; evidence aktual di bawah.

- Document: nearest supported root/skip two-way/boundary/one-root, Source bytes/
  identity/revision/cache pointer/caret/checkpoint/history invariants, cold/no warm/
  ten forged targets/no mutation, single/nested/literal forged roots, EOF/indent/
  oversized skip, fresh offsets/snapshot setelah Apply/Undo/Redo/replace, Source
  redo dan delayed save checkpoint tetap utuh.
- Controller: Preview/Split neighbor flags/raw payload/caret0/unchanged Source,
  dirty selection/history retained +notice tidak memblokir Apply +Undo→move resets
  old local redo, boundary/invalid/parser refusal no loss, stale/Source/prompt/Open/
  other modal/no-draft guards, navigated no-op Apply/Cancel preserve Source redo,
  second-only atomic Apply/caret/Undo/Redo, immutable Save/Export +logical search/
  Go-to context/replies. Semua tes lama dipertahankan.
- UI: bounds/error/Message direction/notice does not disable Apply, clearing notice
  only on effective edit/history restore, fresh empty session. Core grammar/history
  scanners unchanged, tidak menambah tes trivial enum ke portable count.

## Suite GUI privat17

`--suite paragraph-navigation`: app/Xvfb/XDG/env/clipboard privat baru,180s alarm/
5min step tersendiri, job45min unchanged. **27 old exercise bodies AST-identical
against `a616dd2`**; semua16existing gates tetap. Tidak mengganti byte/coordinate/
oracle/deadline/refocus lama; no retry/runtime hooks.

**19 checks PASS** =18 exact Copy (8Source+10draft)+natural clean Close0:

1. Welcome/fixture dua root Unicode/raw hard breaks dipisahkan heading/single-line/
   nested/fenced content. Open first, native draft Copy/focus, changed draft Next
   refusal preserves raw text/selection/focus; Ctrl+Z kembali unchanged.
2. Next ke exact second supported root, melewati blok lain; Ctrl+Y tidak membawa
   redo sesi lama. Previous kembali first. Disabled first Previous→Escape/Cancel
   mempertahankan exact Source, tanpa mengklik input untuk refocus.
3. Split open→Next second raw/focus. Disabled last Next→Escape/Cancel tidak wrap/
   Apply. Reopen→Next, stage Unicode/trailing spaces second; dirty Previous refused,
   notice tidak men-disable explicit Apply.
4. Apply lalu native X tanpa Source refocus di last payload end; Source typing dan
   entire second-paragraph Apply dua Undo terpisah, seluruh first/skipped neighbors/
   suffix intact. Satu Undo lagi kembali welcome clean, natural client Close0:
   navigation/cancel/boundary bukan hidden Source edges.

Native Copy membuktikan raw text/focus; selection retention diuji controller,
bukan native selection-anchor/direction/pixel oracle.12click hanya product Open/Previous/Next/Apply (termasuk disabled boundaries), bukan
refocus workaround. Evidence `paragraph-navigation/paragraph-navigation-preview.png`,
`paragraph-navigation-dirty.png` bukan pixel/glyph oracle. Observed aggregate
**329 GUI** =299clipboard(252Source+47draft),12titles,18natural exits.

```sh
./scripts/test-gui-x11.sh --suite paragraph-navigation --artifacts "$(mktemp -d)"
# Tetap jalankan semua16suite lama, bukan substitusi.
```

## Status eksekusi

Local format/harness format,**294strict core**,**57Python**,diff check PASS.
[CI 39886493920](https://github.com/Indrawan007/Hylite/actions/runs/39886493920),
`13769acbbcca76ef055566b9035265b990faf7cf`, **success**, watcher exit0;
job `120195796902`,23 Oktober2026 **01:53:10–02:02:24 UTC** /
**09:53:10–10:02:24 Makassar**. Native format/strict Clippy/debug build/**869Cargo**
dan seluruh17GUI PASS pada feature push pertama; no calibration/test-only fix.
Check annotations melaporkan116/9/9/9/14/16/11/11/11/11/11/11/16/15/16/24/19
=**329GUI**,869Cargo. Core/Python counts dari local same-revision runs; langkah
CI terkait juga PASS, bukan tambahan unique native Rust tests.

Artifact `12473142607`, `gui-x11-smoke`, **3.106.610bytes**, unexpired saat dicek;
API SHA256 `226db022da74c356f646598d171cda009fc87c3a3f652a067a48b77db3a544b8`,
expires2026-10-30T02:02:20Z. Evidence dari run/job/steps/check annotations dan
artifact metadata, bukan full archived-log/visual screenshot inspection.
Docs7c run39885686359 cancelled superseded feature7d;7c feature39884654030 success.
No main merge.

## Manual / batas

- [ ]Banyak eligible roots/offscreen,128line/16KiB navigation UI, other unsupported
  variants/list/fallback, native selection direction, button traversal/narrow
  window/disabled blur/accessibility, physical query/Go-to focus/scroll/layout.
- [ ]Arch/Hyprland/Wayland/portal/IME/RTL/HiDPI/GPU. X11 tiny-skia bukan target
  compositor proof; screenshot belum direview visual.

Belum Apply & Next/Previous paragraph, cross-kind navigation, EOF/blank/indented
editing, implicit Apply/create/reorder/delete, local history single-line/cell,
rich selection/glyph mapping/free inline WYSIWYG/multi-block selection/sync scroll/
incremental parse/virtualization/performance claim. Source authoritative, full
candidate parser pada Apply, bounded Source history/barriers dan file pipeline tetap.
