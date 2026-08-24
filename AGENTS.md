# Orbis Agent Guide

## Project Overview

Orbis is a procedural planet renderer and simulation project written in Rust.
The current application is a native desktop program built with `winit` and `wgpu`.
The long-term rendering and simulation direction is documented in `PLAN.md`.

Keep implementations simple and observable while the rendering foundation is still developing.

## Repository Structure

```text
.
├── .github/
│   ├── workflows/ci.yml          # Pull request and main branch checks
│   ├── dependabot.yml            # Automated dependency updates
│   └── pull_request_template.md  # Pull request description format
├── src/
│   ├── app.rs                    # Native application and window lifecycle
│   ├── main.rs                   # Executable entry point
│   └── renderer/
│       ├── mod.rs                # High-level rendering behavior
│       └── surface.rs            # GPU and presentation-surface lifecycle
├── Cargo.lock                    # Locked dependency versions
├── Cargo.toml                    # Package metadata and dependencies
├── justfile                      # Common development commands
├── PLAN.md                       # Project goals and technical roadmap
├── README.md                     # Project introduction
└── rust-toolchain.toml           # Pinned Rust toolchain and components
```

## Architecture

Keep `src/main.rs` limited to process startup and top-level error propagation.
Keep native event-loop and window-lifecycle behavior in `src/app.rs`.
Keep high-level rendering behavior in `src/renderer/mod.rs`.
Keep GPU initialization, surface recovery, resizing, and presentation in `src/renderer/surface.rs`.
Add modules only when a concrete responsibility needs its own boundary.
Keep the project as one crate until multiple crates provide a clear architectural benefit.

Prefer synchronous implementations until measurement shows that asynchronous work is needed.
Measure CPU, GPU, memory, and synchronization costs before adopting more complex rendering or compute approaches.

## Rust Guidelines

Use the Rust edition and exact toolchain declared by the repository.
Prefer clear ownership and explicit error propagation over hidden global state.
Avoid `unsafe` code unless it is necessary, narrowly scoped, and documented with its safety requirements.

## Development Commands

Use the `justfile` commands so local checks match CI.

```sh
just run        # Run the native application
just fmt        # Format Rust source files
just fmt-check  # Check formatting without changing files
just lint       # Run Clippy with warnings denied
just test       # Run all tests
just check      # Run formatting, linting, and tests
```

Run `just check` before handing off code changes.
Running the application requires a graphical desktop environment.

## Testing and CI

Add focused tests for behavior that can be tested without a native window or GPU.
Keep platform-specific application code small so core logic can be tested independently as the project grows.
CI must pass formatting, Clippy, and tests with all targets and features enabled.

## Code Review Standards

Focus review comments on concrete bugs, regressions, security issues, flaky behavior, and meaningful maintainability risks.
Attach each finding to the smallest relevant code range.
Use one short paragraph to explain what can fail, when it can fail, and how to fix it.
Avoid praise, summaries, style-only suggestions, duplicate findings, and speculative concerns.
If there are no actionable findings, say so without inventing comments.

## Documentation

Update `README.md` when setup or user-facing behavior changes.
Update `PLAN.md` only when project goals, scope, or architectural direction change.
Keep documentation aligned with the implemented repository structure.
