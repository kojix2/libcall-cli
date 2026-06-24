#!/bin/bash
set -e

echo "=== libcall v2.0 Test Examples ==="
echo

echo "1. Basic math functions:"
./target/release/libcall -lm sqrt 16.0 :f64
./target/release/libcall -lm pow 2.0 3.0 :f64
echo

echo "2. String operations:"
./target/release/libcall -lc strlen "hello world" :usize
echo

echo "3. System calls:"
./target/release/libcall -lc getpid :i32
echo

echo "4. Output parameters (modf):"
./target/release/libcall -lm modf f64:3.14 @f64 :f64
echo

echo "5. JSON output:"
./target/release/libcall --format json -lm sqrt 16.0 :f64
echo

echo "6. Dry run mode:"
./target/release/libcall --dry-run -lm pow 2.0 3.0 :f64
echo

echo "All tests passed!"
