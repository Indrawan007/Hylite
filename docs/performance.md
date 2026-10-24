# Stage 10b — benchmark harness

**Harness and first baseline complete.** This page records raw absolute timings
from one CI run. It intentionally draws no performance conclusion and makes no
speed claim.

## Scope and repeatability

The opt-in, headless test in `src/performance_benchmarks.rs` creates deterministic
ASCII Markdown documents of exactly **100,000 bytes (100 KB), 1,000,000 bytes
(1 MB), and 5,000,000 bytes (5 MB)**. The original fixtures contain repeated
paragraphs with headings and inline emphasis, code, and link syntax. The two
Stage 10a observations use separate plain-paragraph fixtures at the same sizes;
that keeps the original baseline workload intact. Each metric has one warm-up
followed by three measured samples; the table reports each set as `min / median
/ max`, in milliseconds.

| Metric | Timed operation | Deliberately excluded |
|---|---|---|
| `open` | Read a temporary local file and initialize `Document` from its contents. The warm-up means measured reads are expected to be OS-cache reads. | App startup, window creation, async file-I/O scheduling, and cold-storage latency |
| `typing` | Insert one ASCII `x` at the initial caret via `Document::apply`; includes the editor's history/revision work for that edit. | `Document` setup, UI event dispatch, and preview parsing |
| `parse` | Cold `Document::preview_blocks()` projection; includes native-buffer text extraction and Markdown parsing. | `Document` setup and rendering |
| `render_tree` | Construct the read-only Preview Iced `Element` tree from already-parsed blocks. | Layout, renderer paint, GPU, window-system presentation, and frame latency |
| `incremental_preview` | After a full warm parse, insert one ASCII `x` into the isolated plain first paragraph, then time the cache-miss `preview_blocks()` call. Includes native-buffer snapshot, exact Source diff discovery, one-line parse, and cloning/shifting retained suffix blocks. | Setup, initial full parse, UI dispatch, layout/rendering |
| `fallback_preview` | After a full warm parse, insert a newline at the start of the same plain fixture, then time `preview_blocks()`; the changed block is no longer isolated and must use the full-parser fallback. Includes the same cache-miss/source-snapshot work as the incremental metric. | Setup, initial full parse, UI dispatch, layout/rendering |

## First raw CI baseline

Recorded on **2026-10-24**, GitHub Actions run
[40086623422](https://github.com/Indrawan007/Hylite/actions/runs/40086623422),
commit `39e81681474c3785f91f9507abfca58bd27934ad`. Environment: GitHub-hosted
`ubuntu-24.04`, stable Rust toolchain (the workflow tracks `stable`, not a pinned
version), Cargo `test` profile (`unoptimized + debuginfo`). The explicitly run
benchmark test completed in 3.17 s including fixture creation and all samples.

| Operation (ms: min / median / max) | 100 KB | 1 MB | 5 MB |
|---|---:|---:|---:|
| `open` | 0.070 / 0.072 / 0.086 | 2.153 / 2.192 / 3.038 | 33.835 / 34.274 / 36.315 |
| `typing` | 0.056 / 0.057 / 0.096 | 0.614 / 0.640 / 0.653 | 4.204 / 4.255 / 4.353 |
| `parse` | 5.000 / 5.014 / 5.018 | 50.078 / 50.157 / 50.189 | 249.613 / 249.968 / 250.914 |
| `render_tree` | 0.192 / 0.197 / 0.205 | 1.920 / 1.968 / 1.980 | 9.566 / 9.586 / 9.711 |

These are observations from one runner and one run, not portable guarantees,
comparative results, regression thresholds per operation, or evidence of actual
window rendering. The CI guard is a 180-second whole-suite assertion plus a
5-minute step timeout; it is a coarse fail-safe against stalls or extreme
regressions, not a latency target.

The test is `#[ignore]` in the ordinary Cargo suite and CI invokes it explicitly.
To run it locally:

```sh
cargo test --locked --bin hylite --all-features \
  performance_benchmarks::stage10b_baseline -- --ignored --exact --nocapture
```

Measurements are environment-specific and should only be compared with runs
from a sufficiently similar environment. This harness is not a pixel, GPU,
desktop-target, or general performance proof. No speed claim is made from these
numbers.
