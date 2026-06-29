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

[audit]
# hides findings from `pulse audit`

[history]
# tunes `pulse history`
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
global_conditionals_max = 0
global_nesting_depth = 3

# duplication
duplication_min_loc = 6
skeleton_duplication_min_loc = 20
duplication_min_group = 2
duplication_min_distinct_kinds = 3

# aggregate smells
large_fn_loc = 40
large_fn_count = 3
consecutive_asserts_max = 10
primitive_ratio_threshold = 0.7
primitive_min_typed_params = 4
primitive_min_same_count = 2
constructor_dep_injection_min = 4
framework_dep_injection_min = 8
lcom4_warning = 3
short_var_min_fn_loc = 15
short_var_max_count = 3
max_string_match_arms = 5
dup_assert_min = 6
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
  "hallucinated_import",
  "cross_file_duplication",
  "unused_function",
  "dead_store",
  "use_before_def",
  "unreachable_code",
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

## Detector Threshold Tables

Sub-tables under `[thresholds]` tune the `pulse audit` detectors and the opt-in dataflow smells. All keys are optional.

```toml
# opt-in dataflow smells (off by default)
[thresholds.cpg]
enabled = false
dead_store = true
use_before_def = false
unreachable_code = true

# class smells
[thresholds.named_smells.god_class]
wmc = 47
tcc = 0.33
atfd = 5

[thresholds.named_smells.refused_bequest]
enabled = false

[thresholds.named_smells.multivariate_anomaly]
enabled = true
min_classes = 8
distance_quantile = 11.143
max_findings = 10

# cross-file pattern mining
[thresholds.pattern_mining]
corpus_idiom_frequency = 0.10   # opt-in: suppress patterns common across reference codebases

# also available: [thresholds.taint], [thresholds.clone_cluster],
# [thresholds.naturalness], [thresholds.package_metrics]
```

## Audit Configuration

`pulse audit` is an experimental surface. The `[audit]` table hides findings from its output. It does not change what gets detected.

```toml
[audit]
hide_smells = ["feature_envy", "god_class"]
hide_categories = ["literal_repetition", "method_call"]
hide_patterns = ["*logger*", "build_*_response"]
```

`hide_smells` drops named-smell, architecture, dependency, and security findings by slug:

```
shotgun_surgery  divergent_change  feature_envy
god_class  parallel_inheritance  refused_bequest
low_conceptual_cohesion  multivariate_anomaly

distance_from_main_sequence  import_cycle  unstable_dependency
hub_like_dependency  god_component  over_fragmentation
compound_arch_smell  split_component  move_file
merge_components  zero_edge_project

bloated_dependency  phantom_dependency  undeclared_module_dependency
unused_declared_dependency  constraint_smell  strictness_debt
outdated_dependency  vulnerable_dependency

ifdef_density  injection_shape  near_duplicate
unnatural_code  vulnerable_clone_sibling
```

`hide_categories` drops cross-file pattern findings by category:

```
primitive_obsession  chained_dict_access  enum_value_access
attribute_chain  literal_repetition  method_call
comparison  assignment  dict_literal  list_literal  other
```

`hide_patterns` drops cross-file pattern findings whose snippet matches a glob. The globs run against the snippet text, not file paths.

`[ignore] paths` applies to audit as well. Matching files are never walked.

## History Configuration

`pulse history` is an experimental surface. It reads the `[history]` table.

```toml
[history]
ignore_paths = ["migrations/**", "*.lock"]

[history.co_change]
max_findings = 20

[history.hotspot]
max_findings = 10

[history.contributors]
max_findings = 20

[history.jit]
use_lt = true
use_age = true
use_entropy = true
```

`ignore_paths` adds to `[ignore] paths` for history only, using the same glob syntax.

`max_findings` caps how many findings each pass reports. Defaults are 20 for co-change, 10 for hotspot, 20 for contributors.

The CLI flags `--co-change-top`, `--hotspot-top`, and `--contributors-top` override the matching `max_findings`. `--hist` and `--arch-trend` enable the opt-in evolutionary and cycle-trend passes; `--no-szz` disables the on-by-default defect-prone-file pass (bug-fix blame); `--jit-calibrate` writes JIT edit-risk calibration for the repo. `--since`, `--max-commits`, and `--root` have no `.pulse.toml` equivalent.
