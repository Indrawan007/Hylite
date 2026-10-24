# Stage 10b — benchmark harness

**Status: harness added; initial CI timing run is pending.** This page will record
raw absolute timings from the first successful Stage 10b CI run. It intentionally
does not draw a performance conclusion or make a speed claim.

## Scope and repeatability

The opt-in, headless test in `src/performance_benchmarks.rs` creates deterministic
ASCII Markdown documents of exactly **100,000 bytes (100 KB), 1,000,000 bytes
(1 MB), and 5,000,000 bytes (5 MB)**. The fixtures contain repeated paragraphs
with headings and inline emphasis, code, and link syntax. Each metric has one
warm-up followed by three measured samples; CI reports each sample set as
`min/median/max`, in milliseconds.

| Metric | Timed operation | Deliberately excluded |
|---|---|---|
| `open` | Read a temporary local file and initialize `Document` from its contents. The warm-up means measured reads are expected to be OS-cache reads. | App startup, window creation, async file-I/O scheduling, and cold-storage latency |
| `typing` | Insert one ASCII `x` at the initial caret via `Document::apply`; includes the editor's history/revision work for that edit. | `Document` setup, UI event dispatch, and preview parsing |
| `parse` | Cold `Document::preview_blocks()` projection; includes native-buffer text extraction and Markdown parsing. | `Document` setup and rendering |
| `render_tree` | Construct the read-only Preview Iced `Element` tree from already-parsed blocks. | Layout, renderer paint, GPU, window-system presentation, and frame latency |

The test is `#[ignore]` in the ordinary Cargo suite and CI invokes it explicitly.
To run it locally:

```sh
cargo test --locked --bin hylite --all-features \
  performance_benchmarks::stage10b_baseline -- --ignored --exact --nocapture
```

A **180-second suite budget** is asserted by the harness. The dedicated CI step
also has a **5-minute hard timeout**, including Cargo/test-process overhead. This
is a coarse guard against stalls or extreme regressions, not a per-operation
latency target. Measurements are environment-specific and should only be
compared with runs from a sufficiently similar CI environment; this harness is
not a pixel, GPU, desktop-target, or general performance proof.

## Initial CI baseline

Pending the first successful run of the explicit Stage 10b CI step. Values will
be recorded with the run/commit and environment once available; until then, no
benchmark measurements or performance claims are asserted.
