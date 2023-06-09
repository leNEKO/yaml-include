# Yaml-include

A yaml processor that can recursivly include files through `!include <path>` tag.

> well, it kinda works with json as well see [data/simple/other.json](data/simple/other.json)

## Install

```shell
cargo install yaml-include
```

## Usage

### Help

```shell
yaml-include --help
```

```yaml
Output yaml with recursive "!included" data

Usage: yaml-include <FILE_PATH>

Arguments:
  <FILE_PATH>  main yaml file to process

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Run

Ex.:

```shell
yaml-include data/sample/main.yml > main_inlined.yml
```

## Features

- include and parse recursivly `yaml` (and `json`) files
- include `markdown` and `txt` text files
- include other types as `base64` encoded binary data. 

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

> see [data/sample](data/sample)
