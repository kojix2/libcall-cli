# libcall v2.0

Call C functions from shared libraries directly from the command line.

## Overview

**libcall** is a command-line tool that allows you to invoke C functions from shared libraries (`.so`, `.dylib`, `.dll`) without writing any code. It's designed for shell scripting, CI/CD pipelines, and quick testing of low-level APIs.

**Key Features:**
- **Type Inference**: Minimal syntax with automatic type detection
- **Shell-Friendly**: zsh-compatible syntax (no `[]`, `*`, `?`, `{}` conflicts)
- **Rust-Style Types**: Concise type names (`i32`, `f64`, `usize`, etc.)
- **Array Support**: Input, output, and inout arrays with length-prefix notation
- **Output Parameters**: Dedicated `@` prefix for output parameters
- **Multiple Output Formats**: Human-readable, JSON, and YAML (planned)

## Installation

### Build from Source

```bash
git clone https://github.com/kojix2/rust-libcall.git
cd libcall
cargo build --release
sudo cp target/release/libcall /usr/local/bin/
```

## Quick Start

### Basic Function Calls

```bash
# Call sqrt(16.0) from libm
libcall -lm sqrt 16.0 :f64
# Output: 4.0

# Call pow(2.0, 3.0)
libcall -lm pow 2.0 3.0 :f64
# Output: 8.0

# Call strlen("hello")
libcall -lc strlen "hello" :usize
# Output: 5

# Get process ID
libcall -lc getpid :i32
# Output: 12345

# Print a message
libcall -lc puts "Hello, World!" :i32
# Output: Hello, World!
#         14
```

## Command-Line Syntax

```
libcall [OPTIONS] <LIBRARY> <FUNCTION> [ARGS...] [:RETURN_TYPE]
```

### Options

| Option              | Description                                  | Example               |
| ------------------- | -------------------------------------------- | --------------------- |
| `-l NAME` or `-lNAME` | Specify library name (searches for `libNAME`)| `-lm`, `-lc`     |
| `-L PATH` or `-LPATH` | Add library search path (multiple allowed)   | `-L/opt/lib`         |

### Options

| Option              | Description                                  | Example               |
| ------------------- | -------------------------------------------- | --------------------- |
| `-l NAME`           | Specify library name (searches for `libNAME`)| `-lm`, `-lSystem`     |
| `-L PATH`           | Add library search path (multiple allowed)   | `-L /opt/lib`         |
| `--output FORMAT`   | Output format: json, yaml, or human (default)| `--output json`       |
| `--verbose`         | Show verbose information                     | `--verbose`           |
| `--dry-run`         | Parse arguments without executing            | `--dry-run`           |
| `-h, --help`        | Show help message                            | `--help`              |
| `-v, --version`     | Show version information                     | `--version`           |

### Library Specification

Three ways to specify a library:

1. **Full path**: `/usr/lib/libm.so.6`
2. **Relative path**: `./libtest.so`
3. **`-l` form**: `-lm` (searches standard paths for `libm`)

**Library Search Order:**
1. Paths specified with `-L`
2. Environment variables (`LD_LIBRARY_PATH`, `DYLD_LIBRARY_PATH` on macOS)
3. System default paths:
   - Linux: `/lib`, `/usr/lib`, `/lib/x86_64-linux-gnu`, `/usr/lib/x86_64-linux-gnu`
   - macOS: `/usr/lib`, `/usr/local/lib`, `/opt/homebrew/lib`
   - Windows: `C:\Windows\System32`, `C:\Windows\SysWOW64`

## Type System

### Type Inference

Values are automatically inferred from literals:

| Literal           | Inferred Type | Notes                         |
| ----------------- | ------------- | ----------------------------- |
| `123`             | `i32`         | Integer literal               |
| `123.45`          | `f64`         | Floating-point literal        |
| `"hello"`         | `cstr`        | String literal (null-terminated) |
| `true` / `false`  | `i32`         | Boolean (treated as C int)    |
| `null`            | `ptr`         | NULL pointer                  |

### Explicit Type Specification

Use `type:value` format when type inference is not appropriate:

```bash
f32:16.0         # Treat as float
i64:123          # 64-bit integer
u8:255           # 8-bit unsigned integer
cstr:PATH        # String "PATH"
ptr:null         # NULL pointer
```

### Supported Types

| Type (Recommended) | Aliases                              | C Type                | Size (bits) |
| ------------------ | ------------------------------------ | --------------------- | ----------- |
| `i8`               | `char`, `int8`, `int8_t`             | `int8_t`              | 8           |
| `u8`               | `uchar`, `uint8`, `uint8_t`, `byte`  | `uint8_t`             | 8           |
| `i16`              | `short`, `int16`, `int16_t`          | `int16_t`             | 16          |
| `u16`              | `ushort`, `uint16`, `uint16_t`       | `uint16_t`            | 16          |
| `i32`              | `int`, `int32`, `int32_t`            | `int32_t`             | 32          |
| `u32`              | `uint`, `uint32`, `uint32_t`         | `uint32_t`            | 32          |
| `i64`              | `long_long`, `int64`, `int64_t`      | `int64_t`             | 64          |
| `u64`              | `ulong_long`, `uint64`, `uint64_t`   | `uint64_t`            | 64          |
| `isize`            | `ssize`, `ssize_t`, `long`           | `ssize_t`             | 32/64       |
| `usize`            | `size`, `size_t`, `ulong`            | `size_t`              | 32/64       |
| `f32`              | `float`                              | `float`               | 32          |
| `f64`              | `double`                             | `double`              | 64          |
| `cstr`             | `string`, `str`, `char*`             | `const char*`         | ptr         |
| `ptr`              | `pointer`, `voidp`, `void*`          | `void*`               | ptr         |
| `callback`         | `func`, `function`                   | function pointer      | ptr         |
| `void`             | -                                    | `void` (return only)  | -           |

### Array Types

#### Input Arrays (Immutable)

```
Ntype:val1,val2,...
```

- `N`: Number of elements (positive integer)
- `type`: Element type
- Values are comma-separated

**Examples:**
```bash
4i32:1,2,3,4              # int32_t[4] = {1, 2, 3, 4}
5f64:1.0,2.0,3.0,4.0,5.0  # double[5] = {1.0, ..., 5.0}
3cstr:foo,bar,baz         # const char*[3] = {"foo", "bar", "baz"}
```

#### Output Arrays (Mutable)

```
@Ntype
```

- `@`: Prefix indicating output parameter
- Array where the function writes values

**Examples:**
```bash
@16u8       # uint8_t out[16] (for output)
@4f64       # double out[4]
```

#### Inout Arrays (Mutable with Initializer)

```
@Ntype:val1,val2,...
```

- Has initial values that the function may overwrite

**Examples:**
```bash
@4i32:4,2,3,1    # int32_t arr[4] = {4,2,3,1}; qsort(arr, ...)
```

### Callback Functions (Phase 3 - NEW!)

Callback functions are written in Lua and can be passed to C functions like `qsort`.

#### Syntax

```
'return_type(arg_types){ lua_body }'
```

**Important:** The callback string must be quoted with single quotes inside the shell argument.

**Examples:**
```bash
# Sort an array with qsort
libcall -l c qsort '@4i32:4,2,3,1' 'usize:4' 'usize:4' \
  "'i32(ptr a, ptr b){ return i32(a) - i32(b) }'" :void
# Output: [0] 4i32 = [1, 2, 3, 4]

# Reverse sort
libcall -l c qsort '@4i32:1,2,3,4' 'usize:4' 'usize:4' \
  "'i32(ptr a, ptr b){ return i32(b) - i32(a) }'" :void
# Output: [0] 4i32 = [4, 3, 2, 1]
```

#### Lua Environment

Callback functions have access to the following helper functions:

| Function          | Description                              | Example                 |
| ----------------- | ---------------------------------------- | ----------------------- |
| `i8(ptr)`         | Read int8_t from pointer                 | `i8(a)`                 |
| `u8(ptr)`         | Read uint8_t from pointer                | `u8(a)`                 |
| `i16(ptr)`        | Read int16_t from pointer                | `i16(a)`                |
| `u16(ptr)`        | Read uint16_t from pointer               | `u16(a)`                |
| `i32(ptr)`        | Read int32_t from pointer                | `i32(a)`                |
| `u32(ptr)`        | Read uint32_t from pointer               | `u32(a)`                |
| `i64(ptr)`        | Read int64_t from pointer                | `i64(a)`                |
| `f32(ptr)`        | Read float from pointer                  | `f32(a)`                |
| `f64(ptr)`        | Read double from pointer                 | `f64(a)`                |
| `cstr(ptr)`       | Read C string from pointer               | `cstr(a)`               |
| `write_i32(p, v)` | Write int32_t to pointer                 | `write_i32(out, 42)`    |
| `write_f64(p, v)` | Write double to pointer                  | `write_f64(out, 3.14)`  |

#### Callback Examples

```bash
# Compare integers (ascending)
"'i32(ptr a, ptr b){ return i32(a) - i32(b) }'"

# Compare integers (descending)
"'i32(ptr a, ptr b){ return i32(b) - i32(a) }'"

# More complex logic
"'i32(ptr a, ptr b){ 
    local x = i32(a)
    local y = i32(b)
    if x > y then return 1
    elseif x < y then return -1
    else return 0 end
}'"
```

### Return Type

Return type is specified with `:type` format. Defaults to `void` if omitted.

```bash
libcall -lm sqrt 16.0 :f64           # Return type is double
libcall -lc getpid :i32              # Return type is int32_t
libcall -lc puts "hello" :i32        # Return type is int32_t
libcall -lc free ptr:0x12345 :void   # No return value (explicit)
```

## Examples

### Mathematical Functions

```bash
# Square root
libcall -lm sqrt 16.0 :f64
# Output: 4.0

# Power function
libcall -lm pow 2.0 3.0 :f64
# Output: 8.0

# Absolute value
libcall -lc abs i32:-42 :i32
# Output: 42
```

### String Operations

```bash
# String length
libcall -lc strlen "hello world" :usize
# Output: 11

# Get environment variable
libcall -lc getenv cstr:PATH :cstr
# Output: /usr/local/bin:/usr/bin:/bin:...

# Print to stdout
libcall -lc puts "Hello, libcall!" :i32
# Output: Hello, libcall!
#         15
```

### Output Parameters

```bash
# modf: split floating-point number into integer and fractional parts
libcall -lm modf f64:3.14 @f64 :f64
# Output: 0.14000000000000012
#         Outputs:
#           [1] f64 = 3.0
```

### Array Operations

```bash
# Random bytes (Linux getrandom)
libcall -lc getrandom @16u8 usize:16 u32:0 :isize
# Output: 16
#         Outputs:
#           [0] 16u8 = [0x3a, 0x7f, 0x12, ..., 0xc4]
```

### Callback Functions

```bash
# Sort array with qsort
libcall -l c qsort '@4i32:4,2,3,1' 'usize:4' 'usize:4' \
  "'i32(ptr a, ptr b){ return i32(a) - i32(b) }'" :void
# Output: Outputs:
#           [0] 4i32 = [1, 2, 3, 4]

# Reverse sort
libcall -l c qsort '@5i32:5,1,4,2,3' 'usize:5' 'usize:4' \
  "'i32(ptr a, ptr b){ return i32(b) - i32(a) }'" :void
# Output: Outputs:
#           [0] 5i32 = [5, 4, 3, 2, 1]
```

### macOS System Libraries

```bash
# Get process ID on macOS
libcall -lSystem getpid :i32

# Get environment variable on macOS
libcall -lSystem getenv cstr:HOME :cstr

# Print message on macOS
libcall -lSystem puts "Hello from macOS" :i32
```

### JSON Output

```bash
libcall --output json -lm sqrt 16.0 :f64
```

Output:
```json
{
  "library": "/usr/lib/libm.so.6",
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

### YAML Output

```bash
libcall --output yaml -l m sqrt 16.0 :f64
```

Output:
```yaml
args:
- type: f64
  value: 16.0
function: sqrt
library: /usr/lib/libm.so.6
outputs: []
return:
  type: f64
  value: 4.0
```

### Dry Run Mode

```bash
libcall --dry-run -lm pow 2.0 3.0 :f64
```

Output:
```
Library: /usr/lib/libm.so.6
Function: pow
Return type: f64
Arguments:
  [0] f64 (output: false)
  [1] f64 (output: false)
```

## Error Handling

### Library Not Found

```bash
$ libcall -lnonexistent foo
Error: Library not found: libnonexistent
Searched paths:
  - /lib
  - /usr/lib
  - /usr/local/lib
```

### Function Not Found

```bash
$ libcall -lm nonexistent_func
Error: Symbol not found: nonexistent_func (...)
```

### Type Conversion Error

```bash
$ libcall -lm sqrt "invalid" :f64
Error: invalid float literal
```

### Array Length Mismatch

```bash
$ libcall ./test.so func 4i32:1,2,3 ...
Error: Array length mismatch: expected 4 elements, got 3
```

## Platform Support

- **Linux**: Full support (glibc, musl)
- **macOS**: Full support (including dyld shared cache)
- **Windows**: Basic support (DLL loading)

## Architecture

**libcall v2** is implemented in Rust with the following components:

- **Type System** (`types.rs`): Type definitions and value conversions
- **Parser** (`parser.rs`): Command-line argument parsing with regex
- **Library Resolver** (`library.rs`): Dynamic library loading and symbol resolution
- **FFI Executor** (`ffi.rs`): Foreign function interface calls using direct trampolines
- **Output Formatter** (`output.rs`): Human-readable and JSON output
- **Callback Bridge** (`callback.rs`): Lua-based callback support (planned)

## Development Status

### Implemented Features (Phase 1 & 2 & 3)
- ✅ Basic function calls with scalar arguments
- ✅ Type inference (integer, float, string)
- ✅ Explicit type specification (`type:value`)
- ✅ Return type specification (`:type`)
- ✅ Library search (`-l`, `-L`)
- ✅ Error handling (library/function not found)
- ✅ Input arrays (`Ntype:values`)
- ✅ Output arrays (`@Ntype`)
- ✅ Inout arrays (`@Ntype:values`)
- ✅ **Lua callback functions** (NEW in Phase 3!)
- ✅ **Callback integration with qsort** (NEW in Phase 3!)
- ✅ JSON output format
- ✅ **YAML output format** (NEW in Phase 3!)
- ✅ Verbose and dry-run modes

### Planned Features (Phase 4)
- ⏳ YAML/JSON file input for complex calls  
- ⏳ Struct support (future)
- ⏳ Interactive mode (future)

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## License

MIT License - see LICENSE file for details.

## Links

- **Repository**: https://github.com/kojix2/rust-libcall
- **Specification**: See [SPEC_V2.md](SPEC_V2.md) for detailed technical specification

## Version

Current version: **2.0.0**

## Authors

- kojix2
