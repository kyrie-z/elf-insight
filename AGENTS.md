# AGENTS.md

Instructions for AI agents working in this repository. Keep it short and factual; update it when a command or convention changes.

## What this repo is

`elf-insight` is a Rust terminal UI (ratatui + crossterm) for inspecting ELF binaries: overview, layout map, hexdump, disassembly, strings, dynamic entries. It is a single binary crate, not a workspace.

## Toolchain

- Rust, edition 2021, stable channel. There is no `rust-toolchain.toml` and no declared MSRV.
- Verified with rustc 1.95.0 / cargo 1.95.0 on 2026-10-31.
- Dependencies: `ratatui` 0.29, `crossterm` 0.28, `goblin` 0.9, `iced-x86` 1.21 — see `Cargo.toml`.

## Commands

| Purpose | Command |
| --- | --- |
| Build (debug) | `cargo build` |
| Build (release) | `cargo build --release` |
| Test | `cargo test` |
| Run | `cargo run -- <path-to-elf>` |
| Format check | `cargo fmt --check` |
| Lint | `cargo clippy --all-targets` |

What is actually enforced:

- There is no CI in this repo. `cargo build --release` and `cargo test` (29 tests) pass on `master`; those are the gates to report.
- `cargo fmt --check` currently fails on `master` — the tree is not rustfmt-clean. Do not reformat unrelated files.
- `cargo clippy --all-targets` exits 0 but reports 25 warnings on `master` (2026-10-31). Do not add new warnings, and do not run `cargo clippy --fix` across the repo inside an unrelated change.

## Tests

- Unit tests live inline in `#[cfg(test)]` modules beside the code:
  - `src/elf/parser.rs`
  - `src/elf/disasm.rs`
  - `src/ui/strings.rs`
  - `src/ui/tree.rs`
- There is no `tests/` integration directory.
- `elf::parser::tests::test_parse_elf_on_real_binary` parses `/bin/ls`, so it only runs on Linux-like systems with a standard `/bin/ls`.
- New tests go in a `#[cfg(test)]` module at the bottom of the file you changed, following the existing naming style (`test_<behaviour>`).

## Layout

- `src/main.rs` — entry point and CLI argument handling
- `src/app.rs` — app state machine, event dispatch, view routing
- `src/elf/` — ELF parsing (`parser.rs`, goblin) and disassembly (`disasm.rs`, iced-x86)
- `src/ui/` — one module per view; `mod.rs` renders the split layout
- `docs/superpowers/` — design and plan notes, historical only; not build inputs

## Branch and commit conventions

- Base branch: `master`. Never commit or push directly to `master`.
- Working branch: `agent/vest-<issue-number>-<slug>` (for example `agent/vest-3-agent-brief`), one branch per issue. The Multica checkout may start you on `agent/mika/<run-id>`; create the `agent/vest-...` branch from it before committing.
- Commits: Conventional Commits — `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`. Imperative mood, one logical change per commit. The existing history follows this style.

## Pull requests

- Open a PR (draft is fine while in progress) against `master`.
- Title: same Conventional Commits style as the commits.
- Description must state what changed and why, how it was verified (the exact commands and their results, including skipped or failed checks), and link the Multica issue (`VEST-<n>`).
- Keep the diff to the issue. No drive-by refactors, dependency bumps, or formatting sweeps.

## Never

- Never merge a PR. The member reviews and merges.
- Never force-push or rewrite history on a shared branch.
- Never hand-edit `Cargo.lock`; change `Cargo.toml` and let cargo update it.
- Never commit build output — `target/` is ignored, along with `ls` and `.claude/` (see `.gitignore`).
- Never add a CI workflow or a new lint gate as a side effect of an unrelated change.

## Verifying a change locally

1. `cargo build --release` — must succeed.
2. `cargo test` — must pass.
3. For UI changes: this is a TUI, so exercise it with `cargo run -- /bin/ls` in a real terminal and say what you checked. A headless run cannot confirm rendering; state that limitation instead of claiming visual verification.
