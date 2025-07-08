# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust project demonstrating the typestate pattern for a compiler pipeline. The codebase implements a state machine where each compilation stage (Scanner, Parser, Evaluater, CompilerResult) is enforced at compile time through generic type parameters.

## Common Commands

- Build: `cargo build`
- Run: `cargo run`
- Check: `cargo check`
- Test: `cargo test`
- Format: `cargo fmt`
- Lint: `cargo clippy`

## Architecture

The core architecture uses the typestate pattern to enforce compilation pipeline stages:

1. **Compiler<S>** - Generic struct that holds the current stage state
2. **Stage Types** - Scanner, Parser, Evaluater, CompilerResult represent different compilation phases
3. **State Transitions** - Each stage can only transition to the next valid stage:
   - Scanner → Parser (via `scan()`)
   - Parser → Evaluater (via `parse()`)
   - Evaluater → CompilerResult (via `evaluate()`)

The pattern prevents invalid state transitions at compile time, ensuring the compiler pipeline is used correctly.

## Key Files

- `src/main.rs` - Complete implementation with all stage types and transitions
- `Cargo.toml` - Uses Rust 2024 edition with no external dependencies