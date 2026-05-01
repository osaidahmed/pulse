# `.pulse.toml`

Put `.pulse.toml` in your project root. Pulse searches upward from the checked file and uses the closest config.

```toml
[thresholds]
arg_max = 8
fn_loc_warning = 80

[disable]
smells = ["primitive_obsession"]

[languages.python]
cc_warning = 12
fn_loc_warning = 90

[languages.go]
arg_max = 7
```

## Sections

```toml
[thresholds]
# global defaults for every language

[languages.python]
# overrides only Python

[disable]
# disables selected smells everywhere

[ignore]
# skips matching paths entirely
```

## Ignore Paths

```toml
[ignore]
paths = [
  "vendor/**",
  "third_party/**",
  "**/generated/**",
  "legacy_module",
  "*.gen.py",
]
```

Patterns are matched against each file's path **relative to the directory containing `.pulse.toml`**. Globbing follows standard syntax:

- `*` — match a single path segment (no `/`)
- `**` — match any number of segments, including zero
- `?` — match a single character
- `[abc]` — match any character in the set
- `{a,b}` — match either alternative

A bare folder name like `legacy_module` is treated as both `legacy_module` and `legacy_module/**` so the whole folder is skipped. Trailing `/` is also accepted (e.g. `legacy_module/`).

When a file matches, pulse produces no findings, caches no baselines, logs no analytics, and budget/debug commands report it as `ignored by .pulse.toml`.

## Language Keys

```toml
[languages.python]
[languages.typescript]
[languages.javascript]
[languages.rust]
[languages.c]
[languages.cpp]
[languages.java]
[languages.csharp]
[languages.go]
[languages.swift]
[languages.zig]
[languages.ruby]
[languages.objc]
[languages.tcl]
[languages.kotlin]
[languages.haskell]
[languages.lua]
[languages.r]
[languages.php]
[languages.cobol]
[languages.d]
[languages.groovy]
```

## Threshold Keys

```toml
[thresholds]

# complexity
cc_warning = 9
cc_alert = 18
cogc_warning = 15
cogc_alert = 25

# function size/shape
fn_loc_warning = 65
fn_loc_alert = 100
arg_max = 5
constructor_arg_max = 5
nesting_depth = 4
bump_count = 2
compound_conditions = 2
embedded_block_loc = 15

# file/module size
file_loc_warning = 500
file_loc_alert = 700
file_function_count = 20
file_total_cc = 100
max_declarations = 20
max_struct_fields = 12

# duplication
duplication_min_loc = 6
skeleton_duplication_min_loc = 20
duplication_min_group = 2

# aggregate smells
large_fn_loc = 40
large_fn_count = 3
consecutive_asserts_max = 10
primitive_ratio_threshold = 0.7
primitive_min_typed_params = 4
lcom4_warning = 3
short_var_min_fn_loc = 15
short_var_max_count = 3
max_string_match_arms = 5
```

## Disable Smells

```toml
[disable]
smells = [
  "god_method",
  "complex_method",
  "large_method",
  "nested_conditional_chunks",
  "deep_nested_complexity",
  "complex_conditional",
  "excess_arguments",
  "constructor_over_injection",
  "large_embedded_block",
  "primitive_obsession",
  "large_assertion_block",
  "empty_error_handler",
  "file_too_large",
  "too_many_functions",
  "overall_code_complexity",
  "god_class",
  "excessive_declarations",
  "global_conditionals",
  "deep_global_nesting",
  "code_duplication",
  "duplicated_assertion_blocks",
  "low_cohesion",
  "overall_function_size",
  "large_struct",
  "short_variable_names",
  "stringly_typed_switch",
]
```

## Examples

```toml
# Python project, less strict about function size
[languages.python]
fn_loc_warning = 90
fn_loc_alert = 130
```

```toml
# Existing codebase, start by disabling noisy checks
[disable]
smells = ["primitive_obsession", "short_variable_names"]
```

```toml
# Go project, allow more function arguments
[languages.go]
arg_max = 7
constructor_arg_max = 8
```
