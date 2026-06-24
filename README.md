# libcall

`libcall` calls C functions from shared libraries from the command line.

It can be useful for quick checks, scripts, and experiments with C APIs.

Safety: `libcall` runs native code in the current process. Lua callback bodies also run in the current process. Use only trusted libraries, spec files, and callback code.

## Install

```bash
git clone https://github.com/kojix2/rust-libcall.git
cd rust-libcall
cargo build --release
sudo cp target/release/libcall /usr/local/bin/
```

## Basic Use

```bash
libcall -lm sqrt 16.0 :f64
# 4

libcall -lm pow 2.0 3.0 :f64
# 8

libcall -lc strlen "hello" :usize
# 5

libcall -lc abs i32:-42 :i32
# 42

libcall -lc puts "hello" :i32
# hello
# <return value>
```

On macOS, use `-lSystem` when `-lc` is not the library name you want:

```bash
libcall -lSystem getpid :i32
```

## Syntax

```text
libcall [OPTIONS] <LIBRARY> <FUNCTION> [ARGS...] [:RETURN_TYPE]
libcall [OPTIONS] -l NAME <FUNCTION> [ARGS...] [:RETURN_TYPE]
```

If no return type is given, `void` is used.

Common options:

| Option | Meaning |
| --- | --- |
| `-l NAME`, `-lNAME` | Search for a library such as `libm`, `libc`, or `libSystem` |
| `-L PATH`, `-LPATH` | Add a library search path |
| `--format human|json` | Select output format |
| `--spec FILE` | Load a `.json` call spec |
| `--dry-run` | Parse and show the call without running it |
| `--verbose` | Print extra information |

Library search uses `-L`, platform library path environment variables, and system library paths.

## Values And Types

Simple values are inferred:

| Input | Type |
| --- | --- |
| `123` | `i32` |
| `3000000000` | `i64` |
| `123.45` | `f64` |
| `hello` | `cstr` |
| `true`, `false` | `i32` |
| `null` | `ptr` |

Use `type:value` when inference is not what you want:

```bash
i64:123
u8:255
f32:1.5
cstr:PATH
ptr:null
```

Supported scalar types:

```text
i8 u8 i16 u16 i32 u32 i64 u64 isize usize f32 f64 cstr ptr void
```

Useful aliases include `int` for `i32`, `long` for `isize`, `size_t` for `usize`, `double` for `f64`, and `char*` for `cstr`.

## Arrays

Input array:

```bash
libcall ./libexample.so sum 4i32:1,2,3,4 usize:4 :i32
```

Output array:

```bash
@16u8
```

Inout array:

```bash
@4i32:4,2,3,1
```

Example using `bzero`:

```bash
libcall -lc bzero '@4u8:1,2,3,4' usize:4 :void
# Outputs:
#   [0] 4u8 = [0x00, 0x00, 0x00, 0x00]
```

## Output Formats

Human output is the default.

```bash
libcall --format json -lm sqrt 16.0 :f64
```

Example JSON output:

```json
{
  "library": "libm.dylib",
  "function": "sqrt",
  "args": [
    {
      "type": "f64",
      "value": 16.0
    }
  ],
  "return": {
    "type": "f64",
    "value": 4.0
  },
  "outputs": []
}
```

## Spec Files

Use `--spec` for calls stored in JSON.

```json
{
  "library": "m",
  "function": "sqrt",
  "args": [
    {
      "type": "f64",
      "value": 16.0
    }
  ],
  "returns": "f64"
}
```

Run it:

```bash
libcall --spec call.json
```

Spec libraries without `/`, `\`, or `.` are treated like `-l` names. For example, `"library": "m"` searches for libm.

## Lua Callbacks

Callback support is intentionally narrow. Currently `libcall` supports one callback argument per call, and only this callback shape:

```text
i32(ptr, ptr)
```

This is enough for `qsort` comparators.

```bash
libcall -lc qsort '@4i32:4,2,3,1' usize:4 usize:4 \
  "'i32(ptr a, ptr b){ return i32(a) - i32(b) }'" :void
# Outputs:
#   [0] 4i32 = [1, 2, 3, 4]
```

Lua helper functions available inside callbacks:

```text
i8(ptr) u8(ptr) i16(ptr) u16(ptr) i32(ptr) u32(ptr) i64(ptr)
f32(ptr) f64(ptr) cstr(ptr)
write_i32(ptr, value) write_f64(ptr, value)
```

Unsupported callback signatures fail before the C function is called.

## Dry Run

```bash
libcall --dry-run -lm pow 2.0 3.0 :f64
```

Example output:

```text
Library: libm.dylib
Function: pow
Return type: f64
Arguments:
  [0] f64 (output: false)
  [1] f64 (output: false)
```

## Limitations

`libcall` uses a hand-written FFI dispatcher. It supports common scalar, pointer, array, and qsort-style callback calls, but it is not a complete C ABI engine.

Not supported:

- structs and unions
- variadic functions
- arbitrary callback signatures
- multiple callback arguments in one call
- every possible argument type and arity combination

## Errors

Examples:

```bash
libcall -lnonexistent foo
# Error: Library not found: libnonexistent

libcall -lm nonexistent_func
# Error: Symbol not found: nonexistent_func (...)

libcall -lm sqrt f64:invalid :f64
# Error: invalid float literal
```

## License

MIT
