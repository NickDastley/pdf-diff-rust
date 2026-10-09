# Status

**Last updated:** 2026-10-09
**State:** foundation complete (core domain crate implemented and tested); PDF
extraction, diffing, and the CLI are stubs.

This file is the handoff/continuation note for the project. Read it first when
picking the work back up.

---

## What this project is

`pdf-diff` compares two PDF files structurally and reports what changed:
text blocks that were added, removed, modified, or moved, plus image changes.
It is organised as a Cargo workspace of small, single-responsibility crates,
with the goal of being an easy-to-read, well-tested, open-source (Apache-2.0)
library and CLI.

## Repository layout

```
pdf-diff-rust/
├── Cargo.toml                  # virtual workspace manifest
├── rust-toolchain.toml         # pinned toolchain + components
├── rustfmt.toml                # formatting rules
├── .editorconfig
├── .vscode/                    # editor settings + recommended extensions
├── docs/
│   └── STATUS.md               # this file
└── crates/
    ├── pdfdiff-core/           # IR types, config, ids, text  (IMPLEMENTED)
    ├── pdfdiff-extract/        # `Extractor` trait + errors    (stub)
    ├── pdfdiff-diff/           # `DiffError`                   (stub)
    └── pdfdiff-cli/            # `pdf-diff` binary             (stub)
```

**Dependency direction (acyclic, enforced by the compiler):**

```
pdfdiff-cli ─▶ pdfdiff-extract ─▶ pdfdiff-core
     └──────▶ pdfdiff-diff ────▶ pdfdiff-core
```

## Build, test, lint

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p pdfdiff-cli
```

The toolchain is pinned, so `rustup` uses the right compiler automatically.

## What is implemented

`pdfdiff-core` is complete and fully tested (23 unit tests + 2 integration
tests):

- **`model`** — the document IR (`DocumentIr`, `PageMeta`, `Element`,
  `ElementContent`, `ElementLayout`, `TextBlockContent`, `ImageContent`,
  `TableContent`), plus `sorted_elements` (deterministic reading order) and
  `DocumentIr::index_by_id`. Serde attributes define the on-disk JSON shape.
- **`text`** — `normalize_text`: NFC normalisation, removal of soft hyphen and
  zero-width space, whitespace collapsing.
- **`ids`** — `sha1_hex`, `round_half_to_even`, and `build_element_id`
  (deterministic element identifiers).
- **`config`** — `DiffConfig` with defaults, TOML `[diff]` loading, `PDF_DIFF_*`
  environment overrides (environment > TOML > defaults), strict `validate()`, and
  `to_dict()`.
- **`error`** — `ConfigError`.

The JSON emitted by the IR types is intentionally stable: it is a compatibility
contract for downstream consumers (a future HTML viewer and HTTP API).

## What is not implemented yet

- **Extraction** (`pdfdiff-extract`): turning a PDF into a `DocumentIr` via a
  concrete `Extractor` backend, plus recurring header/footer detection.
- **Diffing** (`pdfdiff-diff`): text matching, token/character spans, move and
  image detection, and the diff JSON assembly.
- **CLI** (`pdfdiff-cli`): `extract`, `compare`, and `diff` subcommands.
- **Tests fixtures / golden tests**, CI, and packaging.

## Conventions

- Rust edition 2024; formatting via `rustfmt`; lints via workspace
  `[lints]` and `clippy -D warnings`.
- Library crates expose typed errors with `thiserror`; the binary will use
  `anyhow` at the top level.
- Determinism matters: sorting is explicit and serialized maps have stable order.

## Suggested next steps

1. Add a concrete `Extractor` backend in `pdfdiff-extract` behind the existing
   trait, producing `DocumentIr` values.
2. Add text-block segmentation (glyphs → lines → blocks).
3. Implement the compare pipeline in `pdfdiff-diff`.
4. Wire up the CLI subcommands.
5. Add fixtures and golden/determinism tests; then CI.
