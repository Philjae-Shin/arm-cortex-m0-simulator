/// Usage:
///   cargo run -- [OPTIONS] <program.asm>
///
/// Options:
///   --sim <mode>        Simulator mode: scalar | pipeline | ooo  (default: scalar)
///   --units <N>         Number of execution units / RS (default: 2, for pipeline/ooo)
///   --predictor <kind>  Branch predictor: static | 1bit | 2bit  (default: static)
///   --forwarding        Enable data forwarding in pipelined mode
///   --verbose           Print per-cycle trace
///   --batch             Run program on all three simulators and print comparison table
///   --experiment        Run full experiment suite across all programs in a directory

mod isa;
mod assembler;
mod memory;
mod regs;
mod execute;
mod stats;
mod branch_pred;
mod cache;
mod sim_scalar;
mod sim_pipeline;
mod sim_ooo;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.contains(&"--help".to_string()) {
        print_usage();
        process::exit(0);
    }

    // Parse arguments
    let mut program_path: Option<String> = None;
    let mut sim_mode = "scalar".to_string();
    let mut num_units: usize = 2;
    let mut pred_kind_str = "static".to_string();
    let mut forwarding = false;
    let mut verbose = false;
    let mut batch = false;
    let mut experiment = false;
    let mut use_cache = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--sim" => {
                i += 1;
                if i < args.len() { sim_mode = args[i].clone(); }
            }
            "--units" => {
                i += 1;
                if i < args.len() {
                    num_units = args[i].parse().unwrap_or(2);
                }
            }
            "--predictor" => {
                i += 1;
                if i < args.len() { pred_kind_str = args[i].clone(); }
            }
            "--forwarding" => { forwarding = true; }
            "--cache" => { use_cache = true; }
            "--verbose" | "-v" => { verbose = true; }
            "--batch" => { batch = true; }
            "--experiment" => { experiment = true; }
            s if !s.starts_with('-') => {
                program_path = Some(s.to_string());
            }
            other => {
                eprintln!("Unknown option: {}", other);
                process::exit(1);
            }
        }
        i += 1;
    }

    let pred_kind = branch_pred::PredictorKind::from_str(&pred_kind_str)
        .unwrap_or_else(|| {
            eprintln!("Unknown predictor: {}. Using 'static'.", pred_kind_str);
            branch_pred::PredictorKind::Static
        });

    // Experiment mode
    if experiment {
        run_experiments(pred_kind, verbose);
        return;
    }

    // Single-program / Batch mode
    let path = program_path.unwrap_or_else(|| {
        eprintln!("Error: no program file specified.");
        print_usage();
        process::exit(1);
    });

    if batch {
        run_batch(&path, num_units, pred_kind, forwarding, verbose);
    } else {
        run_single(&path, &sim_mode, num_units, pred_kind, forwarding, verbose, use_cache);
    }
}

// Run a single program with one simulator (111111)
fn run_single(
    path: &str,
    sim_mode: &str,
    num_units: usize,
    pred_kind: branch_pred::PredictorKind,
    forwarding: bool,
    verbose: bool,
    use_cache: bool,
) {
    let mut mem = memory::Memory::new();
    let program = assembler::load_from_file(path, &mut mem).unwrap_or_else(|e| {
        eprintln!("Assembly error: {}", e);
        process::exit(1);
    });

    println!("Program  : {} ({} instructions)", program.name, program.instructions.len());
    println!("Simulator: {}", sim_mode);
    if sim_mode != "scalar" {
        println!("Exec units: {}", num_units);
        println!("Predictor : {}", pred_kind.name());
        if forwarding { println!("Forwarding: enabled"); }
        if use_cache { println!("L1 Cache  : enabled (256-line direct-mapped)"); }
    }
    println!("---");

    let mut regs = regs::RegFile::new(0xFFFC);
    let cache_opt = if use_cache { Some(cache::Cache::new()) } else { None };

    let stats = match sim_mode {
        "scalar" => sim_scalar::run(&program.instructions, &mut regs, &mut mem, verbose),
        "pipeline" => sim_pipeline::run_with_cache(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, forwarding, verbose, cache_opt,
        ),
        "ooo" => sim_ooo::run_with_cache(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, verbose, cache_opt,
        ),
        _ => {
            eprintln!("Unknown simulator mode: {}", sim_mode);
            process::exit(1);
        }
    };

    println!("{}", stats);
}

// Run one program through all three simulators for comparison (222222)
fn run_batch(
    path: &str,
    num_units: usize,
    pred_kind: branch_pred::PredictorKind,
    forwarding: bool,
    verbose: bool,
) {
    println!("Batch Comparison: {}\n", path);
    println!("{:<14} {:>8} {:>8} {:>8} {:>10} {:>10}",
        "Simulator", "Cycles", "Instrs", "IPC", "Br.Pred%", "Stalls");
    println!("{}", "-".repeat(70));

    let modes = [
        ("scalar", 1usize),
        ("pipeline-1", 1),
        ("pipeline-N", num_units),
        ("ooo", num_units),
    ];

    for (label, units) in modes {
        // Fresh state for each run
        let mut mem = memory::Memory::new();
        let program = assembler::load_from_file(path, &mut mem).unwrap_or_else(|e| {
            eprintln!("Assembly error: {}", e);
            process::exit(1);
        });
        let mut regs = regs::RegFile::new(0xFFFC);

        let stats = match label {
            "scalar" => sim_scalar::run(&program.instructions, &mut regs, &mut mem, verbose),
            "pipeline-1" => sim_pipeline::run(
                &program.instructions, &mut regs, &mut mem,
                1, pred_kind, forwarding, verbose,
            ),
            "pipeline-N" => sim_pipeline::run(
                &program.instructions, &mut regs, &mut mem,
                units, pred_kind, forwarding, verbose,
            ),
            "ooo" => sim_ooo::run(
                &program.instructions, &mut regs, &mut mem,
                units, pred_kind, verbose,
            ),
            _ => unreachable!(),
        };

        let pred_pct = if stats.branches_predicted + stats.branches_mispredicted > 0 {
            format!("{:.1}%", stats.prediction_accuracy() * 100.0)
        } else {
            "-".to_string()
        };

        println!("{:<14} {:>8} {:>8} {:>8.4} {:>10} {:>10}",
            label,
            stats.cycles,
            stats.instructions_executed,
            stats.ipc(),
            pred_pct,
            stats.pipeline_stalls,
        );
    }
}

// Run full experiment suite (333333)
fn run_experiments(
    _pred_kind: branch_pred::PredictorKind,
    verbose: bool,
) {
    // Discover all .asm files in /src/programs/ directory
    let programs_dir = "src/programs";
    let asm_files: Vec<String> = match std::fs::read_dir(programs_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_string_lossy().to_string())
            .filter(|p| p.ends_with(".asm"))
            .collect(),
        Err(_) => {
            eprintln!("Cannot read programs/ directory. Trying ./programs/*.asm");
            Vec::new()
        }
    };

    if asm_files.is_empty() {
        eprintln!("No .asm files found in {}/", programs_dir);
        process::exit(1);
    }

    println!(" All 3\n");
    println!("(3 experiments: ISA enhancement, cache extension, SIMD extension)\n");

    // Experiment 1: Post-indexed addressing impact
    println!(" Experiment 1: Post-Indexed Addressing Impact \n");
    let va_post = asm_files.iter().find(|p| {
        let stem = std::path::Path::new(p).file_stem()
            .map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        stem == "vector_add"
    });
    let va_plain = asm_files.iter().find(|p| p.contains("vector_add_plain"));

    if let (Some(post), Some(plain)) = (va_post, va_plain) {
        println!("{:<35} {:>10} {:>10} {:>10} {:>10}",
            "Version / Simulator", "Cycles", "Instrs", "IPC", "Speedup");
        println!("{}", "-".repeat(80));

        for (sim_label, sim_mode, units) in [
            ("scalar", "scalar", 1usize),
            ("pipeline-2", "pipeline", 2),
            ("ooo-4", "ooo", 4),
        ] {
            let s_plain = run_fresh(plain.as_str(), sim_mode, units,
                branch_pred::PredictorKind::TwoBit, true, verbose);
            let s_post = run_fresh(post.as_str(), sim_mode, units,
                branch_pred::PredictorKind::TwoBit, true, verbose);

            if let (Some(sp), Some(spo)) = (s_plain, s_post) {
                let speedup = sp.cycles as f64 / spo.cycles as f64;
                println!("  LDR+ADDI/{:<18} {:>10} {:>10} {:>10.4} {:>10}",
                    sim_label, sp.cycles, sp.instructions_executed, sp.ipc(), "-");
                println!("  LDRPOST/{:<19} {:>10} {:>10} {:>10.4} {:>9.2}x",
                    sim_label, spo.cycles, spo.instructions_executed, spo.ipc(), speedup);
            }
        }
    } else {
        println!("  [skip] vector_add.asm or vector_add_plain.asm not found");
    }

    // Experiment 2: Cache locality analysis
    println!("\n Experiment 2: Cache Locality & Conflict Miss Analysis (Cache Extension) \n");
    let cache_programs: Vec<&String> = asm_files.iter()
        .filter(|p| p.contains("cache_"))
        .collect();

    if !cache_programs.is_empty() {
        println!("{:<25} {:>10} {:>10} {:>10} {:>10} {:>10}",
            "Access Pattern", "No Cache", "Cache", "Hits", "Misses", "Hit Rate");
        println!("{}", "-".repeat(80));

        for path in &cache_programs {
            let name = std::path::Path::new(path).file_stem()
                .map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

            let s_no = run_fresh_cached(path, "pipeline", 2,
                branch_pred::PredictorKind::TwoBit, true, verbose, false);
            let s_cache = run_fresh_cached(path, "pipeline", 2,
                branch_pred::PredictorKind::TwoBit, true, verbose, true);

            if let (Some(nc), Some(wc)) = (s_no, s_cache) {
                let total = wc.cache_hits + wc.cache_misses;
                let hit_rate = if total > 0 {
                    wc.cache_hits as f64 / total as f64 * 100.0
                } else { 0.0 };
                println!("{:<25} {:>10} {:>10} {:>10} {:>10} {:>9.1}%",
                    name, nc.cycles, wc.cycles, wc.cache_hits, wc.cache_misses, hit_rate);
            }
        }
    } else {
        println!("  [skip] No cache_*.asm benchmarks found");
    }

    // Experiment 3: SIMD vector length scaling
    println!("\n Experiment 3: SIMD Vector Length Scaling (SIMD Extension)\n");
    {
        // different vector lengths and compare
        // SIMD (VADD) vs scalar loop part here (1111111)
        let lengths = [2u32, 4, 8, 16, 32];
        println!("{:<8} {:>10} {:>10} {:>10} {:>10} {:>10}",
            "VecLen", "Loop Cyc", "SIMD Cyc", "Loop Instr", "SIMD Instr", "Speedup");
        println!("{}", "-".repeat(65));

        for &len in &lengths {
            // Generate data section with `len` words
            let data_words: Vec<String> = (1..=len).map(|i| i.to_string()).collect();
            let data_b: Vec<String> = (1..=len).map(|i| (i * 2).to_string()).collect();
            let data_c: Vec<String> = vec!["0".to_string(); len as usize];

            let data_a_str = data_words.join(", ");
            let data_b_str = data_b.join(", ");
            let data_c_str = data_c.join(", ");

            let b_offset = 0x1000 + len * 4;
            let c_offset = b_offset + len * 4;

            // SIMD version (uses vector register file V0-V7)
            let simd_src = format!(
                ".data 0x1000\n\
                 arr_a: .word {data_a}\n\
                 arr_b: .word {data_b}\n\
                 arr_c: .word {data_c}\n\
                 .text\n\
                 MOVI R0, #0x1000\n\
                 MOVI R1, #0x{b_off:X}\n\
                 MOVI R2, #0x{c_off:X}\n\
                 VLDR V0, R0, #{len}\n\
                 VLDR V1, R1, #{len}\n\
                 VADD V2, V0, V1, #{len}\n\
                 VSTR V2, R2, #{len}\n\
                 HALT\n",
                data_a = data_a_str, data_b = data_b_str, data_c = data_c_str,
                b_off = b_offset, c_off = c_offset, len = len
            );

            // Scalar loop version
            let loop_src = format!(
                ".data 0x1000\n\
                 arr_a: .word {data_a}\n\
                 arr_b: .word {data_b}\n\
                 arr_c: .word {data_c}\n\
                 .text\n\
                 MOVI R0, #0x1000\n\
                 MOVI R1, #0x{b_off:X}\n\
                 MOVI R2, #0x{c_off:X}\n\
                 MOVI R3, #0\n\
                 MOVI R4, #{len}\n\
                 loop: CMP R3, R4\n\
                 BGE done\n\
                 LDRPOST R5, R0, #4\n\
                 LDRPOST R6, R1, #4\n\
                 ADD R7, R5, R6\n\
                 STRPOST R7, R2, #4\n\
                 ADDI R3, R3, #1\n\
                 B loop\n\
                 done: HALT\n",
                data_a = data_a_str, data_b = data_b_str, data_c = data_c_str,
                b_off = b_offset, c_off = c_offset, len = len
            );

            // Assemble and run both inline
            let s_loop = run_inline(&loop_src, "pipeline", 2,
                branch_pred::PredictorKind::TwoBit, true, verbose);
            let s_simd = run_inline(&simd_src, "pipeline", 2,
                branch_pred::PredictorKind::TwoBit, true, verbose);

            if let (Some(sl), Some(sv)) = (s_loop, s_simd) {
                let speedup = sl.cycles as f64 / sv.cycles as f64;
                println!("{:<8} {:>10} {:>10} {:>10} {:>10} {:>9.2}x",
                    len, sl.cycles, sv.cycles,
                    sl.instructions_executed, sv.instructions_executed, speedup);
            }
        }
    }

    println!("\n[Experiments complete]");
}

/// Helper: run a program with fresh state and return stats.
fn run_fresh(
    path: &str,
    sim_mode: &str,
    num_units: usize,
    pred_kind: branch_pred::PredictorKind,
    forwarding: bool,
    verbose: bool,
) -> Option<stats::Stats> {
    run_fresh_cached(path, sim_mode, num_units, pred_kind, forwarding, verbose, false)
}

fn run_fresh_cached(
    path: &str,
    sim_mode: &str,
    num_units: usize,
    pred_kind: branch_pred::PredictorKind,
    forwarding: bool,
    verbose: bool,
    use_cache: bool,
) -> Option<stats::Stats> {
    let mut mem = memory::Memory::new();
    let program = match assembler::load_from_file(path, &mut mem) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("  [skip] {}: {}", path, e);
            return None;
        }
    };
    let mut regs = regs::RegFile::new(0xFFFC);
    let cache_opt = if use_cache { Some(cache::Cache::new()) } else { None };

    let result = match sim_mode {
        "scalar" => sim_scalar::run(&program.instructions, &mut regs, &mut mem, verbose),
        "pipeline" => sim_pipeline::run_with_cache(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, forwarding, verbose, cache_opt,
        ),
        "ooo" => sim_ooo::run_with_cache(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, verbose, cache_opt,
        ),
        _ => return None,
    };

    Some(result)
}

/// Helper: assemble and run an inline assembly string (for generated experiments).
fn run_inline(
    source: &str,
    sim_mode: &str,
    num_units: usize,
    pred_kind: branch_pred::PredictorKind,
    forwarding: bool,
    verbose: bool,
) -> Option<stats::Stats> {
    let mut mem = memory::Memory::new();
    let program = match assembler::assemble(source, &mut mem) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("  [skip] inline asm: {}", e);
            return None;
        }
    };
    let mut regs = regs::RegFile::new(0xFFFC);

    let result = match sim_mode {
        "scalar" => sim_scalar::run(&program.instructions, &mut regs, &mut mem, verbose),
        "pipeline" => sim_pipeline::run_with_cache(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, forwarding, verbose, None,
        ),
        "ooo" => sim_ooo::run_with_cache(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, verbose, None,
        ),
        _ => return None,
    };

    Some(result)
}

// --help
fn print_usage() {
    eprintln!("ARM Cortex-M0 Processor Simulator");
    eprintln!("Usage: cargo run -- [OPTIONS] <program.asm>");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --sim <mode>         scalar | pipeline | ooo  (default: scalar)");
    eprintln!("  --units <N>          Execution units / reservation stations (default: 2)");
    eprintln!("  --predictor <kind>   static | 1bit | 2bit  (default: static)");
    eprintln!("  --forwarding         Enable data forwarding (pipeline mode)");
    eprintln!("  --cache              Enable L1 data cache simulation");
    eprintln!("  --verbose, -v        Print per-cycle trace");
    eprintln!("  --batch              Run all simulators and print comparison");
    eprintln!("  --experiment         Run full experiment suite on programs/ directory");
}
