use flate2::{Compress, Compression, Decompress, FlushDecompress, Status};
use std::error::Error;
use std::fs;
use std::time::{Duration, Instant};

/// Result of a chunked decompression run.
#[derive(Debug)]
struct Stats {
    runtime: Duration,
    decompressed_size: usize,
}

fn decompress_in_chunks(
    input: &[u8],
    decompressor: &mut Decompress,
    chunk_size: usize,
    scratch_buffer: &mut [u8],
) -> Stats {
    assert!(chunk_size > 0);

    let start = Instant::now();

    let mut total_decompressed = 0usize;

    for mut chunk in input.chunks(chunk_size) {
        while !chunk.is_empty() {
            let before_in = decompressor.total_in();
            let before_out = decompressor.total_out();

            let status = decompressor
                .decompress(chunk, scratch_buffer, FlushDecompress::None)
                .unwrap();

            let consumed = (decompressor.total_in() - before_in) as usize;
            let produced = (decompressor.total_out() - before_out) as usize;

            total_decompressed += produced;

            assert!(
                consumed <= chunk.len(),
                "decompressor reported consuming more input than provided"
            );

            chunk = &chunk[consumed..];

            match status {
                Status::StreamEnd => {
                    let runtime = start.elapsed();
                    return Stats {
                        runtime,
                        decompressed_size: total_decompressed,
                    };
                }
                Status::Ok | Status::BufError => {
                    // BufError can be normal in streaming mode if no progress is possible
                    // with the current buffers; treat it as fatal only if there was no progress.
                    if consumed == 0 && produced == 0 {
                        panic!("decompression made no progress");
                    }
                }
            }
        }
    }

    // After all input has been fed, some formats/streams may still need a final drive.
    loop {
        let before_out = decompressor.total_out();

        let status = decompressor
            .decompress(&[], scratch_buffer, FlushDecompress::Finish)
            .unwrap();
        let produced = (decompressor.total_out() - before_out) as usize;
        total_decompressed += produced;

        match status {
            Status::StreamEnd => break,
            Status::Ok | Status::BufError => {
                if produced == 0 {
                    break;
                }
            }
        }
    }

    Stats {
        runtime: start.elapsed(),
        decompressed_size: total_decompressed,
    }
}

fn compress_in_chunks(
    input: &[u8],
    compressor: &mut flate2::Compress,
    chunk_size: usize,
    scratch_buffer: &mut [u8],
) -> Stats {
    use flate2::{FlushCompress, Status};

    assert!(chunk_size > 0, "chunk_size must be > 0");

    let start = std::time::Instant::now();
    let mut total_compressed = 0usize;

    for mut chunk in input.chunks(chunk_size) {
        while !chunk.is_empty() {
            let before_in = compressor.total_in();
            let before_out = compressor.total_out();

            let status = compressor
                .compress(chunk, scratch_buffer, FlushCompress::None)
                .unwrap();

            let consumed = (compressor.total_in() - before_in) as usize;
            let produced = (compressor.total_out() - before_out) as usize;

            total_compressed += produced;
            chunk = &chunk[consumed..];

            match status {
                Status::Ok | Status::BufError => {
                    assert!(
                        consumed != 0 || produced != 0,
                        "compression made no progress"
                    );
                }
                Status::StreamEnd => {
                    panic!("compress() returned StreamEnd before Finish");
                }
            }
        }
    }

    loop {
        let before_out = compressor.total_out();

        let status = compressor
            .compress(&[], scratch_buffer, FlushCompress::Finish)
            .unwrap();

        let produced = (compressor.total_out() - before_out) as usize;
        total_compressed += produced;

        match status {
            Status::StreamEnd => break,
            Status::Ok | Status::BufError => {
                assert!(produced != 0, "compression made no progress during finish");
            }
        }
    }

    Stats {
        runtime: start.elapsed(),
        decompressed_size: total_compressed,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args();
    let prog = args.next().unwrap();

    let usage = || {
        eprintln!(
            "Usage: {prog} <inflate|deflate> <level> <file> <chunk-size> <repeats> [zlib|raw]"
        )
    };

    enum Op {
        Deflate,
        Inflate,
    }

    let op = match args.next().as_deref() {
        Some("inflate") => Op::Inflate,
        Some("deflate") => Op::Deflate,
        _ => {
            usage();
            std::process::exit(2);
        }
    };

    let level: u32 = match op {
        Op::Inflate => 0,
        Op::Deflate => match args.next() {
            Some(v) => v.parse().unwrap(),
            None => {
                usage();
                std::process::exit(2);
            }
        },
    };

    let Some(input_path) = args.next() else {
        usage();
        std::process::exit(2);
    };

    let Some(chunk_size) = args.next().and_then(|v| v.parse().ok()) else {
        usage();
        std::process::exit(2);
    };

    let Some(repeats) = args.next().and_then(|v| v.parse().ok()) else {
        usage();
        std::process::exit(2);
    };

    let mode = args.next().unwrap_or_else(|| "zlib".to_string());
    let zlib_header = match mode.as_str() {
        "zlib" => true,
        "raw" => false,
        _ => {
            return Err(format!("invalid mode: {mode}; expected 'zlib' or 'raw'").into());
        }
    };

    let input = fs::read(&input_path)?;
    let mut scratch_buffer = vec![0; 64 * 1014];

    let mut compressor = Compress::new(Compression::new(level), zlib_header);
    let mut decompressor = Decompress::new(zlib_header);

    let mut runtimes = Vec::with_capacity(repeats);
    let mut decompressed_size = 0usize;

    for _ in 0usize..repeats {
        compressor.reset();
        decompressor.reset(zlib_header);

        let stats = match op {
            Op::Inflate => {
                decompress_in_chunks(&input, &mut decompressor, chunk_size, &mut scratch_buffer)
            }
            Op::Deflate => {
                compress_in_chunks(&input, &mut compressor, chunk_size, &mut scratch_buffer)
            }
        };
        runtimes.push(stats.runtime);
        decompressed_size = stats.decompressed_size;
    }

    use statrs::statistics::Statistics;

    let total_runtime: Duration = runtimes.iter().sum();
    let mean = runtimes.iter().map(|s| s.as_secs_f64()).mean();
    let stddev = runtimes.iter().map(|s| s.as_secs_f64()).std_dev();

    let secs = total_runtime.as_secs_f64() / repeats as f64;
    let mb = decompressed_size as f64 / (1024.0 * 1024.0);
    let mb_per_sec = if secs > 0.0 { mb / secs } else { 0.0 };

    let ratio = input.len() as f64 / decompressed_size as f64;

    println!("{prog}\n\tmean runtime {mean:.6}s (stdev of {stddev:.6}) at {mb_per_sec:.3} MB/s, ratio of {ratio:.3}");

    Ok(())
}
