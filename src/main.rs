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
        run_single(&path, &sim_mode, num_units, pred_kind, forwarding, verbose);
    }
}

// ---------------------------------------------------------------------------
// Run a single program with one simulator
// ---------------------------------------------------------------------------

fn run_single(
    path: &str,
    sim_mode: &str,
    num_units: usize,
    pred_kind: branch_pred::PredictorKind,
    forwarding: bool,
    verbose: bool,
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
    }
    println!("---");

    let mut regs = regs::RegFile::new(0xFFFC);

    let stats = match sim_mode {
        "scalar" => sim_scalar::run(&program.instructions, &mut regs, &mut mem, verbose),
        "pipeline" => sim_pipeline::run(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, forwarding, verbose,
        ),
        "ooo" => sim_ooo::run(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, verbose,
        ),
        _ => {
            eprintln!("Unknown simulator mode: {}", sim_mode);
            process::exit(1);
        }
    };

    println!("{}", stats);
}

// ---------------------------------------------------------------------------
// Run one program through all three simulators for comparison
// ---------------------------------------------------------------------------

fn run_batch(
    path: &str,
    num_units: usize,
    pred_kind: branch_pred::PredictorKind,
    forwarding: bool,
    verbose: bool,
) {
    println!("=== Batch Comparison: {} ===\n", path);
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

// ---------------------------------------------------------------------------
// Run full experiment suite
// ---------------------------------------------------------------------------

fn run_experiments(
    _pred_kind: branch_pred::PredictorKind,
    verbose: bool,
) {
    // Discover all .asm files in programs/ directory
    let programs_dir = "programs";
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

    println!("=== EXPERIMENT SUITE — ACA Processor Simulator ===\n");

    // Experiment 1: Pipelining benefit
    println!("=== Experiment 1: Pipeline Speedup over Scalar ===\n");
    println!("{:<25} {:>10} {:>10} {:>10} {:>10}",
        "Program", "Scalar", "Pipeline", "Speedup", "IPC(pipe)");
    println!("{}", "-".repeat(70));

    for path in &asm_files {
        let name = std::path::Path::new(path).file_stem()
            .map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

        let s_scalar = run_fresh(path, "scalar", 1, branch_pred::PredictorKind::Static, false, verbose);
        let s_pipe = run_fresh(path, "pipeline", 1, branch_pred::PredictorKind::Static, false, verbose);

        if let (Some(sc), Some(pi)) = (s_scalar, s_pipe) {
            let speedup = sc.cycles as f64 / pi.cycles as f64;
            println!("{:<25} {:>10} {:>10} {:>10.2}x {:>10.4}",
                name, sc.cycles, pi.cycles, speedup, pi.ipc());
        }
    }

    // Experiment 2: Branch predictor comparison
    println!("\n=== Experiment 2: Branch Prediction Comparison ===\n");
    let predictors = [
        branch_pred::PredictorKind::Static,
        branch_pred::PredictorKind::OneBit,
        branch_pred::PredictorKind::TwoBit,
    ];
    println!("{:<25} {:>12} {:>12} {:>12}",
        "Program", "Static", "1-bit", "2-bit");
    println!("{}", "-".repeat(65));

    for path in &asm_files {
        let name = std::path::Path::new(path).file_stem()
            .map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut row = format!("{:<25}", name);
        for pk in &predictors {
            let s = run_fresh(path, "pipeline", 2, *pk, false, verbose);
            if let Some(stats) = s {
                row += &format!(" {:>5} ({:.0}%)", stats.cycles,
                    stats.prediction_accuracy() * 100.0);
            }
        }
        println!("{}", row);
    }

    // Experiment 3: Execution unit scaling (OoO)
    println!("\n=== Experiment 3: Execution Unit Scaling (Out-of-Order) ===\n");
    println!("{:<25} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Program", "1 unit", "2 units", "3 units", "4 units", "6 units");
    println!("{}", "-".repeat(70));

    for path in &asm_files {
        let name = std::path::Path::new(path).file_stem()
            .map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut row = format!("{:<25}", name);
        for units in [1, 2, 3, 4, 6] {
            let s = run_fresh(path, "ooo", units, branch_pred::PredictorKind::TwoBit, false, verbose);
            if let Some(stats) = s {
                row += &format!(" {:>8}", stats.cycles);
            }
        }
        println!("{}", row);
    }

    // Experiment 4: Loop unrolling benefit
    println!("\n=== Experiment 4: Loop Unrolling Benefit ===\n");
    let va_path = asm_files.iter().find(|p| p.contains("vector_add.asm") && !p.contains("unrolled"));
    let va_unrolled = asm_files.iter().find(|p| p.contains("unrolled"));

    if let (Some(orig), Some(unrolled)) = (va_path, va_unrolled) {
        println!("{:<25} {:>10} {:>10} {:>10}",
            "Version", "Cycles", "IPC", "Branches");
        println!("{}", "-".repeat(55));

        for (label, path) in [("vector_add", orig.as_str()), ("vector_add_unrolled", unrolled.as_str())] {
            for (sim_label, sim) in [("scalar", "scalar"), ("pipeline", "pipeline"), ("ooo-4", "ooo")] {
                let units = if sim == "ooo" { 4 } else { 2 };
                let s = run_fresh(path, sim, units, branch_pred::PredictorKind::TwoBit, false, verbose);
                if let Some(stats) = s {
                    println!("{:<15}/{:<9} {:>10} {:>10.4} {:>10}",
                        label, sim_label, stats.cycles, stats.ipc(), stats.branches_total);
                }
            }
        }
    }

    // Experiment 5: Data forwarding impact
    println!("\n=== Experiment 5: Data Forwarding Impact ===\n");
    println!("{:<25} {:>12} {:>12} {:>10}",
        "Program", "No Fwd", "Fwd", "Reduction");
    println!("{}", "-".repeat(62));

    for path in &asm_files {
        let name = std::path::Path::new(path).file_stem()
            .map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

        let s_no_fwd = run_fresh(path, "pipeline", 2, branch_pred::PredictorKind::TwoBit, false, verbose);
        let s_fwd    = run_fresh(path, "pipeline", 2, branch_pred::PredictorKind::TwoBit, true, verbose);

        if let (Some(nf), Some(fw)) = (s_no_fwd, s_fwd) {
            let reduction = (1.0 - fw.cycles as f64 / nf.cycles as f64) * 100.0;
            println!("{:<25} {:>12} {:>12} {:>9.1}%",
                name, nf.cycles, fw.cycles, reduction);
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
    let mut mem = memory::Memory::new();
    let program = match assembler::load_from_file(path, &mut mem) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("  [skip] {}: {}", path, e);
            return None;
        }
    };
    let mut regs = regs::RegFile::new(0xFFFC);

    let result = match sim_mode {
        "scalar" => sim_scalar::run(&program.instructions, &mut regs, &mut mem, verbose),
        "pipeline" => sim_pipeline::run(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, forwarding, verbose,
        ),
        "ooo" => sim_ooo::run(
            &program.instructions, &mut regs, &mut mem,
            num_units, pred_kind, verbose,
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
    eprintln!("  --verbose, -v        Print per-cycle trace");
    eprintln!("  --batch              Run all simulators and print comparison");
    eprintln!("  --experiment         Run full experiment suite on programs/ directory");
}
