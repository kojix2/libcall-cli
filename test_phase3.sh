#!/bin/bash
set -e

echo "=== libcall v2.0 Phase 3 Test Examples ==="
echo

echo "1. Lua Callback - qsort ascending:"
./target/release/libcall -lc qsort '@4i32:4,2,3,1' 'usize:4' 'usize:4' "'i32(ptr a, ptr b){ return i32(a) - i32(b) }'" :void
echo

echo "2. Lua Callback - qsort descending:"
./target/release/libcall -lc qsort '@5i32:5,1,4,2,3' 'usize:5' 'usize:4' "'i32(ptr a, ptr b){ return i32(b) - i32(a) }'" :void
echo

echo "3. YAML output format:"
./target/release/libcall --format yaml -lm sqrt 25.0 :f64
echo

echo "4. YAML output with callback (qsort):"
./target/release/libcall --format yaml -lc qsort '@3i32:3,1,2' 'usize:3' 'usize:4' "'i32(ptr a, ptr b){ return i32(a) - i32(b) }'" :void
echo

echo "5. JSON output with callback:"
./target/release/libcall --format json -lc qsort '@3i32:3,1,2' 'usize:3' 'usize:4' "'i32(ptr a, ptr b){ return i32(a) - i32(b) }'" :void
echo

echo "All Phase 3 tests passed!"
