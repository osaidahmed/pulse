# Pulse

[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.19945787.svg)](https://doi.org/10.5281/zenodo.19945787)

A code smell detector that runs as a Claude Code hook. It analyzes every file edit in real time using tree-sitter, flags structural problems, and blocks the agent until they're fixed.

It also runs standalone: `pulse check <file>` for a file or `pulse check -a` across a project. See [docs/technical-paper.pdf](docs/technical-paper.pdf) for the design and reasoning.

## Install

```sh
brew install osaidahmed/pulse/pulse
```

Or with the shell installer:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/osaidahmed/pulse/releases/latest/download/pulse-installer.sh | sh
```

Then run:

```sh
pulse setup
```

This adds the hooks to `~/.claude/settings.json` and writes instructions to `~/.claude/CLAUDE.md`. Pulse runs automatically in every Claude Code session after that.

Building from source:

```sh
cargo install --path .
```

## Platforms

Tested on macOS, Linux and WSL.

## How it works

Three hooks fire at different points:

- **PostToolUse** (`pulse --hook`): runs after every file edit. Reports function-level findings near the changed lines. Diff-aware, so it won't flag pre-existing problems.
- **Stop** (`pulse --stop`): runs at turn boundary. Detects module-level regressions by comparing against baselines cached at session start.
- **SessionStart** (`pulse --cleanup`): clears stale baselines from previous sessions.

The agent sees findings inline and fixes them before moving on. Target is under 10ms per analysis.

## Smells (26)

Function-level: God Method, Complex Method, Large Method, Nested Conditional Chunks, Deep Nested Complexity, Complex Conditional, Excess Arguments, Constructor Over-Injection, Large Embedded Block, Primitive Obsession, Large Assertion Block, Empty Error Handler, Short Variable Names, Stringly-Typed Switch

Module-level: File Too Large, Too Many Functions, Overall Code Complexity, God Class, Excessive Declarations, Global Conditionals, Deep Global Nesting, Code Duplication (exact + fuzzy), Duplicated Assertion Blocks, Low Cohesion (LCOM4), Overall Function Size, Large Struct

## Languages (22)

Python, TypeScript, JavaScript, Rust, C, C++, Java, C#, Go, Swift, Zig, Ruby, Objective-C, Tcl, Kotlin, Haskell, Lua, R, PHP, COBOL, D, Groovy

## Thresholds

| Metric | Warning | Alert |
|--------|---------|-------|
| Cyclomatic complexity | 9 | 18 |
| Cognitive complexity | 15 | 25 |
| Function LOC | 65 | 100 |
| File LOC | 500 | 700 |

Other limits: nesting depth 4, args 5, compound conditions 2, functions per file 20, total cc per file 100, struct fields 12, string match arms 5.

Full thresholds and justifications: [docs/technical-paper.pdf](docs/technical-paper.pdf).

## Configuration

Drop a `.pulse.toml` in your project root. All fields are optional, defaults apply when absent.

Full syntax: [docs/configuration.md](docs/configuration.md)

```toml
[thresholds]
arg_max = 8
fn_loc_warning = 80

[disable]
smells = ["primitive_obsession"]

[languages.go]
arg_max = 7

[languages.java]
fn_loc_warning = 100
```

## CLI

```
pulse setup              configure hooks and CLAUDE.md
pulse check <file>       full analysis, all findings
pulse check -a           analyze entire project
pulse budget <file>      show remaining headroom
pulse budget --new       show thresholds for a new file
pulse debug <file>       raw metrics dump
pulse audit              cross-file analysis (experimental)
pulse history            git-history mining (experimental)
pulse --hook             PostToolUse hook (reads JSON stdin)
pulse --stop             stop hook (regression detection)
pulse --cleanup          clear baselines
pulse --version, -V      print version
```

## Experimental subcommands

`pulse audit` runs cross-file structural analysis: duplication patterns, god classes, import cycles. `pulse history` mines git history for hotspots and files that keep changing together. I added both because I wanted Pulse to work as a standalone code-smell CLI, not only as an edit hook. Both are rough and produce false positives often enough that you should treat their output as leads to check by hand, not findings to fix. See [docs/configuration.md](docs/configuration.md) for what you can tune and suppress.

## Citation

If you reference Pulse in research, please cite the technical paper:

```bibtex
@misc{saidahmed2026pulse,
  author    = {Saidahmed, Omar},
  title     = {Pulse: Real-Time Structural Feedback for AI Code Generation},
  year      = {2026},
  month     = {May},
  publisher = {Zenodo},
  doi       = {10.5281/zenodo.19945787},
  url       = {https://doi.org/10.5281/zenodo.19945787}
}
```

## License

Apache 2.0
