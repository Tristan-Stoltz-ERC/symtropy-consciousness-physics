use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process;

const HEADER: &str = "trial,first_mode,legacy_ms,deterministic_ms,legacy_rss_kib,deterministic_rss_kib,legacy_stdout_sha256,deterministic_stdout_sha256,legacy_stderr_sha256,deterministic_stderr_sha256";
const BOOTSTRAP_DRAWS: usize = 10_000;

#[derive(Clone)]
struct Row {
    trial: u64,
    legacy_ms: f64,
    deterministic_ms: f64,
    legacy_rss_kib: f64,
    deterministic_rss_kib: f64,
    legacy_stdout: String,
    deterministic_stdout: String,
    legacy_stderr: String,
    deterministic_stderr: String,
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let position = p * (sorted.len() - 1) as f64;
    let lo = position.floor() as usize;
    let hi = position.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let weight = position - lo as f64;
        sorted[lo] * (1.0 - weight) + sorted[hi] * weight
    }
}

fn bootstrap_median_ci(values: &[f64], seed: u64) -> (f64, f64) {
    let mut state = seed;
    let mut samples = Vec::with_capacity(BOOTSTRAP_DRAWS);
    for _ in 0..BOOTSTRAP_DRAWS {
        let mut draw = Vec::with_capacity(values.len());
        for _ in 0..values.len() {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^= z >> 31;
            draw.push(values[(z as usize) % values.len()]);
        }
        samples.push(median(&draw));
    }
    samples.sort_by(f64::total_cmp);
    (percentile(&samples, 0.025), percentile(&samples, 0.975))
}

fn parse(input: &str) -> Result<Vec<Row>, String> {
    let mut lines = input.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected benchmark CSV header".into());
    }

    let mut rows = Vec::new();
    let mut seen_trials = BTreeSet::new();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 10 {
            return Err(format!(
                "line {}: expected 10 columns, got {}",
                offset + 2,
                fields.len()
            ));
        }
        let parse_f64 = |index: usize, label: &str| {
            fields[index]
                .parse::<f64>()
                .map_err(|_| format!("line {}: invalid {}", offset + 2, label))
        };
        let trial = fields[0]
            .parse::<u64>()
            .map_err(|_| format!("line {}: invalid trial", offset + 2))?;
        if trial == 0 || !seen_trials.insert(trial) {
            return Err(format!("line {}: zero or duplicate trial", offset + 2));
        }
        if fields[1] != "legacy" && fields[1] != "deterministic" {
            return Err(format!("line {}: invalid first_mode", offset + 2));
        }
        let legacy_ms = parse_f64(2, "legacy_ms")?;
        let deterministic_ms = parse_f64(3, "deterministic_ms")?;
        let legacy_rss_kib = parse_f64(4, "legacy_rss_kib")?;
        let deterministic_rss_kib = parse_f64(5, "deterministic_rss_kib")?;
        if [
            legacy_ms,
            deterministic_ms,
            legacy_rss_kib,
            deterministic_rss_kib,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x <= 0.0)
        {
            return Err(format!(
                "line {}: measurements must be finite and positive",
                offset + 2
            ));
        }
        for index in 6..10 {
            if fields[index].len() != 64
                || !fields[index].bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(format!(
                    "line {}: invalid SHA-256 in column {}",
                    offset + 2,
                    index + 1
                ));
            }
        }
        rows.push(Row {
            trial,
            legacy_ms,
            deterministic_ms,
            legacy_rss_kib,
            deterministic_rss_kib,
            legacy_stdout: fields[6].to_ascii_lowercase(),
            deterministic_stdout: fields[7].to_ascii_lowercase(),
            legacy_stderr: fields[8].to_ascii_lowercase(),
            deterministic_stderr: fields[9].to_ascii_lowercase(),
        });
    }
    rows.sort_by_key(|r| r.trial);
    if rows.len() < 5 {
        return Err(format!("need at least five paired trials, got {}", rows.len()));
    }
    for (index, row) in rows.iter().enumerate() {
        if row.trial != index as u64 + 1 {
            return Err(format!(
                "trial sequence missing or reordered at index {}",
                index + 1
            ));
        }
    }
    Ok(rows)
}

fn analyze(rows: &[Row]) -> (String, bool) {
    let runtime_ratios: Vec<f64> = rows
        .iter()
        .map(|r| r.legacy_ms / r.deterministic_ms)
        .collect();
    let overhead_percent: Vec<f64> = rows
        .iter()
        .map(|r| (r.deterministic_ms / r.legacy_ms - 1.0) * 100.0)
        .collect();
    let rss_ratios: Vec<f64> = rows
        .iter()
        .map(|r| r.deterministic_rss_kib / r.legacy_rss_kib)
        .collect();
    let rss_delta: Vec<f64> = rows
        .iter()
        .map(|r| r.deterministic_rss_kib - r.legacy_rss_kib)
        .collect();

    let median_speedup = median(&runtime_ratios);
    let median_overhead = median(&overhead_percent);
    let (speedup_low, speedup_high) =
        bootstrap_median_ci(&runtime_ratios, 0x4c55_4d49_4e4f_5553);
    let (overhead_low, overhead_high) =
        bootstrap_median_ci(&overhead_percent, 0x4445_5445_524d_494e);
    let (rss_ratio_low, rss_ratio_high) =
        bootstrap_median_ci(&rss_ratios, 0x5253_535f_424f_4f54);

    let legacy_stdout: BTreeSet<_> = rows.iter().map(|r| &r.legacy_stdout).collect();
    let deterministic_stdout: BTreeSet<_> =
        rows.iter().map(|r| &r.deterministic_stdout).collect();
    let legacy_stderr: BTreeSet<_> = rows.iter().map(|r| &r.legacy_stderr).collect();
    let deterministic_stderr: BTreeSet<_> =
        rows.iter().map(|r| &r.deterministic_stderr).collect();
    let deterministic_repeatable =
        deterministic_stdout.len() == 1 && deterministic_stderr.len() == 1;

    let mut output = String::new();
    output.push_str("# Deterministic reduction paired overhead benchmark\n\n");
    output.push_str(&format!("- Paired trials: {}\n", rows.len()));
    output.push_str("- Speedup is legacy runtime / deterministic runtime; >1 favours deterministic mode.\n");
    output.push_str(&format!(
        "- Median speedup: {:.4}x (95% paired bootstrap CI [{:.4}, {:.4}])\n",
        median_speedup, speedup_low, speedup_high
    ));
    output.push_str(&format!(
        "- Median runtime overhead: {:.3}% (95% paired bootstrap CI [{:.3}%, {:.3}%])\n",
        median_overhead, overhead_low, overhead_high
    ));
    output.push_str(&format!(
        "- Median peak-RSS ratio: {:.4}x (95% paired bootstrap CI [{:.4}, {:.4}])\n",
        median(&rss_ratios), rss_ratio_low, rss_ratio_high
    ));
    output.push_str(&format!(
        "- Median peak-RSS delta: {:.0} KiB\n",
        median(&rss_delta)
    ));
    output.push_str(&format!(
        "- Unique legacy stdout/stderr digests: {}/{}\n",
        legacy_stdout.len(),
        legacy_stderr.len()
    ));
    output.push_str(&format!(
        "- Unique deterministic stdout/stderr digests: {}/{}\n",
        deterministic_stdout.len(),
        deterministic_stderr.len()
    ));
    output.push_str(&format!(
        "- Deterministic repeatability gate: {}\n",
        if deterministic_repeatable {
            "PASS"
        } else {
            "FAIL"
        }
    ));
    output.push_str("\n## Interpretation\n\n");
    output.push_str("- Runtime and peak RSS are measured separately from compilation time.\n");
    output.push_str("- Historical-mode digest variation is diagnostic; differing output is not normalized.\n");
    output.push_str("- The deterministic mode must have one stdout and one stderr digest for the fixed seed schedule.\n");
    output.push_str("- The 3% overhead value is a research target, not a pass/fail gate in this first measurement.\n");
    (output, deterministic_repeatable)
}

fn self_test() -> Result<(), String> {
    let digest = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let mut csv = String::from(HEADER);
    csv.push('\n');
    for trial in 1..=8 {
        csv.push_str(&format!(
            "{},legacy,{},{},{},{},{},{},{},{}\n",
            trial,
            100 + trial,
            103 + trial,
            1000,
            1024,
            digest,
            digest,
            digest,
            digest
        ));
    }
    let rows = parse(&csv)?;
    let (report, gate) = analyze(&rows);
    if !report.contains("Paired trials: 8")
        || !report.contains("Median runtime overhead:")
        || !report.contains("Unique deterministic stdout/stderr digests: 1/1")
        || !gate
    {
        return Err("benchmark analyzer self-test did not emit expected report".into());
    }
    println!("deterministic_reduction_analyzer_self_test=PASS");
    println!("synthetic_paired_rows=8");
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let result = if args.len() == 2 && args[1] == "--self-test" {
        self_test()
    } else if args.len() == 2 {
        fs::read_to_string(&args[1])
            .map_err(|e| format!("cannot read {}: {}", args[1], e))
            .and_then(|content| parse(&content))
            .map(|rows| {
                let (report, deterministic_repeatable) = analyze(&rows);
                print!("{}", report);
                if !deterministic_repeatable {
                    return Err("deterministic binary produced multiple output digests".into());
                }
                Ok(())
            })
            .and_then(|x| x)
    } else {
        Err(format!(
            "usage: {} --self-test | <measurements.csv>",
            args.first().map(String::as_str).unwrap_or("analyzer")
        ))
    };
    if let Err(error) = result {
        eprintln!("benchmark_analysis=REJECTED: {}", error);
        process::exit(2);
    }
}
