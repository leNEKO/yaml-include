# Yaml-include

A cli tool for processing yaml with include documents through `!include <path>` tag.

> it kinda works with json as well see [data/simple/other.json](data/simple/other.json)

## Install

```shell
cargo install yaml-include
```

## Features

- include and parse recursively `yaml` (and `json`) files
- include `markdown` and `txt` text files
- include other types as `base64` encoded binary data.
- by default handle gracefully circular references with `!circular` tag

## Usage

### Help

```shell
yaml-include --help
```

```yaml
A simple cli that output to stdout recursively included data from a yaml file path

Usage: yaml-include [OPTIONS] <FILE_PATH>

Arguments:
  <FILE_PATH>  main yaml file path to process

Options:
  -o, --output-path <OUTPUT_PATH>  optional output path (output to stdout if not set)
  -p, --panic-on-circular          panic on circular reference (default: gracefully handle circular references with !circular tag)
  -h, --help                       Print help
  -V, --version                    Print version
```

### Run

Ex.:

```shell
yaml-include data/sample/main.yml > main_inlined.yml
```

Basically,
turns this:

`main.yml`:

```yaml
data:
    - !include file_a.yml
    - !include file_b.yml
```

`file_a.yml`:

```yaml
something:
    - this
    - that
```

`file_b.yml`:

```yaml
other:
    - text: !include file_c.txt
    - markdown: !include file_d.md
```

`file_c.txt`:

```yaml
This is some "long" multiline
text file i don't want to edit
inline in a long yaml file
```

`file_d.md`:

```markdown
# This is some markdown data

## I don't want to edit

- inline
- in a long yaml file
```

Into that:

```yaml
data:
  - something:
      - this
      - that
  - other:
      - text: |-
          This is some long multiline
          text i don't want to edit
          inline in a long yaml file
      - markdown: |
          # This is some markdown data

          ## I don't want to edit

          - inline
          - in a long yaml file

```

> see [examples/sample](data/sample)
