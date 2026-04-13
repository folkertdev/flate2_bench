cargo build --release --features=miniz_oxide
cp target/release/flate2_bench target/release/flate2_bench_miniz_oxide
cargo build --release --features=zlib-ng
cp target/release/flate2_bench target/release/flate2_bench_zlib_ng
cargo build --release --features=zlib-rs
cp target/release/flate2_bench target/release/flate2_bench_zlib_rs

echo "\n -- inflate (chunks of 4096 bytes) -- \n"

target/release/flate2_bench_miniz_oxide inflate silesia-small.tar.gz 4096 5 zlib
target/release/flate2_bench_zlib_ng inflate silesia-small.tar.gz 4096 5 zlib
target/release/flate2_bench_zlib_rs inflate silesia-small.tar.gz 4096 5 zlib

echo "\n -- deflate level 1 (chunks of 4096 bytes) -- \n"

target/release/flate2_bench_miniz_oxide deflate 1 silesia-small.tar 4096 5 zlib
target/release/flate2_bench_zlib_ng deflate 1 silesia-small.tar 4096 5 zlib
target/release/flate2_bench_zlib_rs deflate 1 silesia-small.tar 4096 5 zlib

echo "\n -- deflate level 6 (chunks of 4096 bytes) -- \n"

target/release/flate2_bench_miniz_oxide deflate 6 silesia-small.tar 4096 5 zlib
target/release/flate2_bench_zlib_ng deflate 6 silesia-small.tar 4096 5 zlib
target/release/flate2_bench_zlib_rs deflate 6 silesia-small.tar 4096 5 zlib

echo "\n -- deflate level 9 (chunks of 4096 bytes) -- \n"

target/release/flate2_bench_miniz_oxide deflate 9 silesia-small.tar 4096 5 zlib
target/release/flate2_bench_zlib_ng deflate 9 silesia-small.tar 4096 5 zlib
target/release/flate2_bench_zlib_rs deflate 9 silesia-small.tar 4096 5 zlib
