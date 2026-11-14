# AlignOn
Align text into columns on one or more delimiters. A small, fast, single static binary written in Rust.
Reads stdin, handles multi-character delimiters, quoted cells and wide (CJK / emoji) characters,
and lets you align **each column left, right or centered** independently.

```
$ printf '1+1=2\n11+11=22\n111+111=222\n' | alignon + = -r
  1 +   1 =   2
 11 +  11 =  22
111 + 111 = 222
```

## Build

```
cargo build --release
# binary: target/release/alignon
```


## Usage

```
alignon [OPTIONS] [DELIM ...]
```

`DELIM` is one or more delimiter strings. With none given the delimiter is **TAB**. Rules:

- When several delimiters could match at the same position, the **longest one wins**.
- `\t` means TAB and `\s` means a single space (quote them in the shell).
- Spaces surrounding a delimiter argument are ignored.
- Cells are trimmed; the aligned output is re-joined with the delimiter and a gap around it.

## Options

Options that take a value use `=` (`-g=2`, not `-g 2`).

| Short | Long | Argument | Description | Default |
|-------|------|----------|-------------|---------|
| `-l` | `--left` | `[=SPEC]` | Left-align the selected columns | all columns |
| `-r` | `--right` | `[=SPEC]` | Right-align the selected columns | off |
| `-c` | `--center` | `[=SPEC]` | Center the selected columns | off |
| `-g` | `--gap` | `=<N>` | Spaces on each side of a delimiter | `1` |
| `-q` | `--qualifier` | `=<QUAL>` | Text qualifier, e.g. `-q='"'`; delimiters inside a qualified cell are not split | none |
| `-f` | `--file` | `=<PATH>` | Read `PATH` instead of stdin | stdin |
| `-o` | `--output` | `=<PATH>` | Write the result to `PATH`; the input file is left untouched | stdout |
| `-i` | `--in-place` | | With `-f`: rewrite the input file in place | off |
| `-h` | `--help` | | Show the built-in colored help | |

### Column SPEC

`-l`, `-r` and `-c` take an optional `=SPEC`. Columns are **1-based**. A bare flag (`-r`) means all columns.

| SPEC | Selects |
|------|---------|
| `1` | column 1 |
| `2-4` | columns 2 to 4 |
| `5-` | column 5 to the last |
| `-3` | columns 1 to 3 |
| `1,3-5,7-` | comma-separated list of any of the above |

- Columns not named by any flag are left-aligned.
- Later flags override earlier ones: `-r -l=2` means every column right-aligned except column 2.
- A range past the widest row is harmless (`-r=1-99`).
- An invalid spec (`-r=x`, `-r=`) exits with an error.

### Text qualifier

With `-q=QUAL`, a cell that starts with `QUAL` is not split until the closing `QUAL`. A doubled `QUAL`
inside is an escaped quote. An unclosed `QUAL` is treated as ordinary text.

## Examples

All examples below are real output.

### Align on several delimiters, right-align everything

```
$ printf '1+1=2\n11+11=22\n111+111=222\n' | alignon + = -r
  1 +   1 =   2
 11 +  11 =  22
111 + 111 = 222
```

### Only the first column right-aligned

```
$ printf '1+1=2\n11+11=22\n111+111=222\n' | alignon + = -r=1
  1 + 1   = 2
 11 + 11  = 22
111 + 111 = 222
```

### Mixed: column 1 right, 2 centered, 3 left

```
$ printf '1+1=2\n11+11=22\n111+111=222\n' | alignon + = -r=1 -c=2 -l=3
  1 +  1  = 2
 11 + 11  = 22
111 + 111 = 222
```

### Quoted cells containing the delimiter

```
$ printf 'name;qty;note\napple;3;"red; sweet"\nfig;12;dried\n' | alignon ';' -q='"'
name  ; qty ; note
apple ; 3   ; "red; sweet"
fig   ; 12  ; dried
```

### Multi-character delimiters (longest match wins)

```
$ printf 'a => 1\nlonger => 22\nx = 3\n' | alignon '=>' '='
a      => 1
longer => 22
x      = 3
```

### Changing the gap around delimiters

```
$ printf 'a,b,c\nlonger,x,yy\n' | alignon , -g=0
a     ,b,c
longer,x,yy

$ printf 'a,b,c\nlonger,x,yy\n' | alignon , -g=3
a        ,   b   ,   c
longer   ,   x   ,   yy
```

### Open-ended range: column 2 to the end right-aligned

```
$ printf 'x|1|2\nlong|22|333\n' | alignon '|' -r=2-
x    |  1 |   2
long | 22 | 333
```

### Wide characters line up by display width

```
$ printf 'name|age\nÅsa|7\n日本語|42\n' | alignon '|' -r=2
name   | age
Åsa    |   7
日本語 |  42
```

## Behaviour notes

- **No trailing whitespace.** The last column is never padded, so left-aligned and centered output has no trailing spaces.
- **Ragged rows are fine.** Rows with fewer cells than the widest row are aligned as far as they go.
- **Colors.** `--help` is colored only when stdout is a terminal and `NO_COLOR` is not set.
- **Exit codes.** `0` on success, `2` for usage or I/O errors (invalid spec, unreadable input, unwritable output).
- A delimiter argument that literally looks like an option (`-l`, `-r`, `-c`, `-r=...`) is treated as an option.
