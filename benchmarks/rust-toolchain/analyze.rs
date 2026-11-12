use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const HEADER: &str = "toolchain,trial,example,build_ms,max_rss_kib,target_bytes,binary_bytes,runtime_repeat,runtime_ms,stdout_sha256";
const BOOTSTRAP_DRAWS: usize = 10_000;

#[derive(Clone, Debug)]
struct Row {
    toolchain: String,
    trial: u64,
    example: String,
    build_ms: u64,
    max_rss_kib: Option<u64>,
    target_bytes: Option<u64>,
    binary_bytes: Option<u64>,
    runtime_repeat: u64,
    runtime_ms: u64,
    stdout_sha256: String,
}

#[derive(Debug)]
struct BuildGroup {
    build_ms: u64,
    max_rss_kib: Option<u64>,
    target_bytes: Option<u64>,
    binary_bytes: Option<u64>,
    runtime_ms: Vec<u64>,
    runtime_repeats: Vec<u64>,
    digests: BTreeSet<String>,
}

fn optional_u64(value: &str, field: &str, line: usize) -> Result<Option<u64>, String> {
    if value == "NA" || value.is_empty() {
        return Ok(None);
    }
    value.parse::<u64>().map(Some).map_err(|_| {
        format!("line {}: {} must be an unsigned integer or NA, got {:?}", line, field, value)
    })
}

fn parse_csv(input: &str) -> Result<Vec<Row>, String> {
    let mut lines = input.lines();
    let header = lines.next().ok_or_else(|| "CSV is empty".to_string())?.trim_end_matches('\r');
    if header != HEADER {
        return Err(format!("unexpected CSV header; expected exactly: {}", HEADER));
    }

    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();

    for (index, raw) in lines.enumerate() {
        let line_number = index + 2;
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if line.contains('"') {
            return Err(format!("line {}: quoted CSV fields are not supported by this numeric schema", line_number));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 10 {
            return Err(format!("line {}: expected 10 fields, found {}", line_number, fields.len()));
        }

        let toolchain = fields[0].to_string();
        let trial = fields[1].parse::<u64>().map_err(|_| format!("line {}: invalid trial", line_number))?;
        let example = fields[2].to_string();
        let build_ms = fields[3].parse::<u64>().map_err(|_| format!("line {}: invalid build_ms", line_number))?;
        let max_rss_kib = optional_u64(fields[4], "max_rss_kib", line_number)?;
        let target_bytes = optional_u64(fields[5], "target_bytes", line_number)?;
        let binary_bytes = optional_u64(fields[6], "binary_bytes", line_number)?;
        let runtime_repeat = fields[7].parse::<u64>().map_err(|_| format!("line {}: invalid runtime_repeat", line_number))?;
        let runtime_ms = fields[8].parse::<u64>().map_err(|_| format!("line {}: invalid runtime_ms", line_number))?;
        let digest = fields[9].to_ascii_lowercase();

        if toolchain.is_empty() || example.is_empty() || trial == 0 || runtime_repeat == 0 {
            return Err(format!("line {}: empty identity or non-positive trial/repeat", line_number));
        }
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("line {}: stdout_sha256 must be 64 hexadecimal characters", line_number));
        }

        let identity = (toolchain.clone(), trial, example.clone(), runtime_repeat);
        if !seen.insert(identity) {
            return Err(format!("line {}: duplicate toolchain/trial/example/runtime_repeat row", line_number));
        }

        rows.push(Row {
            toolchain,
            trial,
            example,
            build_ms,
            max_rss_kib,
            target_bytes,
            binary_bytes,
            runtime_repeat,
            runtime_ms,
            stdout_sha256: digest,
        });
    }

    if rows.is_empty() {
        return Err("CSV contains a header but no measurements".to_string());
    }
    Ok(rows)
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn mad(values: &[f64]) -> f64 {
    let center = median(values);
    let deviations: Vec<f64> = values.iter().map(|value| (value - center).abs()).collect();
    median(&deviations)
}

fn percentile(sorted_values: &[f64], probability: f64) -> f64 {
    let index = ((sorted_values.len() - 1) as f64 * probability).round() as usize;
    sorted_values[index]
}

fn bootstrap_median_ci(values: &[f64]) -> (f64, f64) {
    if values.len() < 2 {
        return (f64::NAN, f64::NAN);
    }

    // Fixed PRNG seed makes the report reproducible for identical input bytes.
    let mut state: u64 = 0x4c55_4d49_4e4f_5553;
    let mut samples = Vec::with_capacity(BOOTSTRAP_DRAWS);
    for _ in 0..BOOTSTRAP_DRAWS {
        let mut draw = Vec::with_capacity(values.len());
        for _ in 0..values.len() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            draw.push(values[(state as usize) % values.len()]);
        }
        samples.push(median(&draw));
    }
    samples.sort_by(f64::total_cmp);
    (percentile(&samples, 0.025), percentile(&samples, 0.975))
}

fn build_groups(rows: &[Row]) -> Result<BTreeMap<(String, u64, String), BuildGroup>, String> {
    let mut groups: BTreeMap<(String, u64, String), BuildGroup> = BTreeMap::new();

    for row in rows {
        let key = (row.toolchain.clone(), row.trial, row.example.clone());
        let group = groups.entry(key).or_insert_with(|| BuildGroup {
            build_ms: row.build_ms,
            max_rss_kib: row.max_rss_kib,
            target_bytes: row.target_bytes,
            binary_bytes: row.binary_bytes,
            runtime_ms: Vec::new(),
            runtime_repeats: Vec::new(),
            digests: BTreeSet::new(),
        });

        if group.build_ms != row.build_ms
            || group.max_rss_kib != row.max_rss_kib
            || group.target_bytes != row.target_bytes
            || group.binary_bytes != row.binary_bytes
        {
            return Err(format!(
                "inconsistent build metrics across repeats: toolchain={} trial={} example={}",
                row.toolchain, row.trial, row.example
            ));
        }
        group.runtime_ms.push(row.runtime_ms);
        group.runtime_repeats.push(row.runtime_repeat);
        group.digests.insert(row.stdout_sha256.clone());
    }

    for ((toolchain, trial, example), group) in &groups {
        let mut repeats = group.runtime_repeats.clone();
        repeats.sort_unstable();
        if repeats.iter().enumerate().any(|(index, repeat)| *repeat != (index as u64 + 1)) {
            return Err(format!("runtime_repeat sequence is not contiguous from 1: toolchain={} trial={} example={}", toolchain, trial, example));
        }
        if group.digests.len() != 1 {
            return Err(format!(
                "determinism failure: {} stdout digests within toolchain={} trial={} example={}",
                group.digests.len(), toolchain, trial, example
            ));
        }
    }
    Ok(groups)
}

fn median_optional(values: &[Option<u64>]) -> Option<f64> {
    let present: Vec<f64> = values.iter().filter_map(|value| value.map(|n| n as f64)).collect();
    if present.is_empty() { None } else { Some(median(&present)) }
}

fn analyze(rows: &[Row], baseline: &str, candidate: &str) -> Result<String, String> {
    if baseline == candidate {
        return Err("baseline and candidate must be different toolchain identifiers".to_string());
    }
    let groups = build_groups(rows)?;
    let toolchains: BTreeSet<String> = rows.iter().map(|row| row.toolchain.clone()).collect();
    for name in [baseline, candidate] {
        if !toolchains.contains(name) {
            return Err(format!("no measurements found for toolchain {:?}", name));
        }
    }

    let examples: BTreeSet<String> = rows
        .iter()
        .filter(|row| row.toolchain == baseline || row.toolchain == candidate)
        .map(|row| row.example.clone())
        .collect();

    let mut report = String::new();
    report.push_str("# Paired Rust toolchain benchmark analysis\n\n");
    report.push_str(&format!("- Baseline: {}\n- Candidate: {}\n", baseline, candidate));
    report.push_str(&format!("- Measurement rows: {}\n", rows.len()));
    report.push_str("- Build metrics are de-duplicated by toolchain/trial/example; runtime repeats are summarized within each build trial.\n");
    report.push_str("- Speedup is baseline time divided by candidate time; values above 1.0 favour the candidate.\n");
    report.push_str("- Confidence intervals use a deterministic percentile bootstrap over paired trial ratios. They are exploratory below 30 paired trials.\n\n");
    report.push_str("| Example | Paired trials | Build median ms (base -> candidate) | Build speedup | Runtime median ms (base -> candidate) | Runtime speedup | 95% bootstrap CI (runtime) | Peak RSS KiB (base -> candidate) | Target bytes (base -> candidate) | Binary bytes (base -> candidate) | Same digest trials |\n");
    report.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");

    for example in examples {
        let mut base_build = BTreeMap::new();
        let mut cand_build = BTreeMap::new();
        for ((toolchain, trial, name), group) in &groups {
            if name != &example { continue; }
            if toolchain == baseline {
                base_build.insert(*trial, group);
            } else if toolchain == candidate {
                cand_build.insert(*trial, group);
            }
        }

        let base_trials: BTreeSet<u64> = base_build.keys().copied().collect();
        let cand_trials: BTreeSet<u64> = cand_build.keys().copied().collect();
        if base_trials != cand_trials {
            return Err(format!("unpaired trial set for {}: baseline={:?}, candidate={:?}", example, base_trials, cand_trials));
        }
        if base_trials.is_empty() {
            return Err(format!("no paired trials for example {}", example));
        }

        let mut base_build_ms = Vec::new();
        let mut cand_build_ms = Vec::new();
        let mut base_runtime_ms = Vec::new();
        let mut cand_runtime_ms = Vec::new();
        let mut build_speedups = Vec::new();
        let mut runtime_speedups = Vec::new();
        let mut base_rss = Vec::new();
        let mut cand_rss = Vec::new();
        let mut base_target_bytes = Vec::new();
        let mut cand_target_bytes = Vec::new();
        let mut base_binary_bytes = Vec::new();
        let mut cand_binary_bytes = Vec::new();
        let mut base_runtime_observations = Vec::new();
        let mut cand_runtime_observations = Vec::new();
        let mut same_digest_trials = 0usize;

        for trial in &base_trials {
            let base = base_build[trial];
            let cand = cand_build[trial];
            let base_run = median(&base.runtime_ms.iter().map(|n| *n as f64).collect::<Vec<_>>());
            let cand_run = median(&cand.runtime_ms.iter().map(|n| *n as f64).collect::<Vec<_>>());

            if base.build_ms == 0 || cand.build_ms == 0 || base_run <= 0.0 || cand_run <= 0.0 {
                return Err(format!("zero-duration measurement cannot be used for ratios ({}, trial {})", example, trial));
            }
            base_build_ms.push(base.build_ms as f64);
            cand_build_ms.push(cand.build_ms as f64);
            base_runtime_ms.push(base_run);
            cand_runtime_ms.push(cand_run);
            build_speedups.push(base.build_ms as f64 / cand.build_ms as f64);
            runtime_speedups.push(base_run / cand_run);
            if base.digests == cand.digests { same_digest_trials += 1; }
            base_runtime_observations.extend(base.runtime_ms.iter().copied());
            cand_runtime_observations.extend(cand.runtime_ms.iter().copied());
            base_rss.push(base.max_rss_kib);
            cand_rss.push(cand.max_rss_kib);
            base_target_bytes.push(base.target_bytes);
            cand_target_bytes.push(cand.target_bytes);
            base_binary_bytes.push(base.binary_bytes);
            cand_binary_bytes.push(cand.binary_bytes);
        }

        let build_speedup = median(&build_speedups);
        let runtime_speedup = median(&runtime_speedups);
        let (ci_low, ci_high) = bootstrap_median_ci(&runtime_speedups);

        report.push_str(&format!(
            "| {} | {} | {:.2} -> {:.2} | {:.3}x (MAD {:.3}) | {:.2} -> {:.2} | {:.3}x (MAD {:.3}) | [{:.3}, {:.3}] | {} -> {} | {} -> {} | {} -> {} | {}/{} |\n",
            example,
            base_trials.len(),
            median(&base_build_ms),
            median(&cand_build_ms),
            build_speedup,
            mad(&build_speedups),
            median(&base_runtime_ms),
            median(&cand_runtime_ms),
            runtime_speedup,
            mad(&runtime_speedups),
            ci_low,
            ci_high,
            median_optional(&base_rss).map(|v| format!("{:.0}", v)).unwrap_or_else(|| "NA".to_string()),
            median_optional(&cand_rss).map(|v| format!("{:.0}", v)).unwrap_or_else(|| "NA".to_string()),
            median_optional(&base_target_bytes).map(|v| format!("{:.0}", v)).unwrap_or_else(|| "NA".to_string()),
            median_optional(&cand_target_bytes).map(|v| format!("{:.0}", v)).unwrap_or_else(|| "NA".to_string()),
            median_optional(&base_binary_bytes).map(|v| format!("{:.0}", v)).unwrap_or_else(|| "NA".to_string()),
            median_optional(&cand_binary_bytes).map(|v| format!("{:.0}", v)).unwrap_or_else(|| "NA".to_string()),
            same_digest_trials,
            base_trials.len()
        ));

        if base_trials.len() < 30 {
            report.push_str(&format!("\n  - {}: exploratory only ({} paired trials); do not make a confirmatory performance claim.\n", example, base_trials.len()));
        }
        if base_runtime_observations.len() < 10_000 || cand_runtime_observations.len() < 10_000 {
            report.push_str(&format!("\n  - {}: p99 not reported (baseline observations={}, candidate observations={}; collect at least 10,000 for each condition).\n", example, base_runtime_observations.len(), cand_runtime_observations.len()));
        } else {
            let base_p99 = quantile_u64(&base_runtime_observations, 0.99).unwrap_or(0);
            let cand_p99 = quantile_u64(&cand_runtime_observations, 0.99).unwrap_or(0);
            report.push_str(&format!("\n  - {}: empirical p99 runtime ms (baseline -> candidate): {} -> {}.\n", example, base_p99, cand_p99));
        }
        if same_digest_trials != base_trials.len() {
            report.push_str(&format!("\n  - {}: output digests differ in {} paired trials; inspect numerical semantics rather than normalizing the difference away.\n", example, base_trials.len() - same_digest_trials));
        }
    }

    report.push_str("\n## Interpretation constraints\n\n");
    report.push_str("- This is an application-workload comparison, not a replacement for upstream rustc-perf.\n");
    report.push_str("- Build speed and simulation runtime are separate outcomes.\n");
    report.push_str("- Equal stdout SHA-256 is a determinism signal, not proof of scientific equivalence.\n");
    report.push_str("- No p99 is reported below 10,000 runtime observations per condition.\n");
    report.push_str("- Compiler promotion still requires compile, test, Clippy, rustdoc, ABI, lockfile and numerical-correctness gates.\n");
    Ok(report)
}

fn self_test() -> Result<(), String> {
    let digest = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let mut csv = String::from(HEADER);
    csv.push('\n');
    for trial in 1..=3 {
        for (toolchain, build_ms, rss, runtime_a, runtime_b) in [
            ("1.96.0", 100, 1000, 50, 60),
            ("1.99.1", 80, 900, 40, 45),
        ] {
            for (repeat, runtime_ms) in [(1, runtime_a), (2, runtime_b)] {
                csv.push_str(&format!(
                    "{},{},hdc_smoke,{},{},2000,500,{}, {},{}\n",
                    toolchain, trial, build_ms, rss, repeat, runtime_ms, digest
                ).replace(", ", ","));
            }
        }
    }

    let rows = parse_csv(&csv)?;
    if rows.len() != 12 { return Err(format!("expected 12 rows, got {}", rows.len())); }
    let report = analyze(&rows, "1.96.0", "1.99.1")?;
    if !report.contains("1.250") { return Err("expected 1.25x build speedup".to_string()); }
    if !report.contains("1.294") { return Err("expected 1.294x runtime speedup".to_string()); }
    if !report.contains("3/3") { return Err("expected identical output digests".to_string()); }
    if !report.contains("p99 not reported") { return Err("small-sample p99 guard missing".to_string()); }

    let invalid = format!("{}\n1.96.0,1,hdc_smoke,100,NA,2000,500,1,50,not-a-digest\n", HEADER);
    if parse_csv(&invalid).is_ok() { return Err("accepted an invalid SHA-256 digest".to_string()); }

    println!("benchmark_analyzer_self_test=PASS");
    println!("paired_trials=3");
    println!("build_speedup_expected=1.25");
    println!("runtime_speedup_expected=1.294");
    println!("deterministic_digest_control=PASS");
    println!("small_sample_p99_guard=PASS");
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let result = if args.len() == 2 && args[1] == "--self-test" {
        self_test()
    } else if args.len() == 4 {
        match fs::read_to_string(&args[1]) {
            Ok(content) => parse_csv(&content)
                .and_then(|rows| analyze(&rows, &args[2], &args[3]))
                .map(|report| print!("{}", report)),
            Err(error) => Err(format!("cannot read {}: {}", args[1], error)),
        }
    } else {
        Err(format!(
            "usage: {} --self-test | <measurements.csv> <baseline-toolchain> <candidate-toolchain>",
            args.first().map(String::as_str).unwrap_or("analyze")
        ))
    };

    if let Err(error) = result {
        eprintln!("benchmark_analysis=REJECTED: {}", error);
        process::exit(2);
    }
}
