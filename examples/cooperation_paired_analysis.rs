// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Opt-in analysis of the existing cooperation_decomposition CSV output.
//!
//! This tool never changes the simulation or its historical verdict. It reads
//! the existing seed-keyed rows and emits a separate machine-readable receipt.

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use symtropy_consciousness_physics::paired_inference::{
    bootstrap_mean_interval, pair_by_seed, paired_sign_flip_test, screen_equivalence,
    summarize_paired, BootstrapConfig, EquivalenceStatus, SeededValue, SignFlipConfig,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("paired analysis failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;
    let input_path = PathBuf::from(required(&args, "--input")?);
    let control_name = required(&args, "--control")?;
    let treatment_name = required(&args, "--treatment")?;
    let metric = required(&args, "--metric")?;
    if control_name == treatment_name {
        return Err("--control and --treatment must name different conditions".into());
    }

    let bootstrap_replicates = parse_usize_option(&args, "--bootstrap-replicates", 20_000)?;
    let bootstrap_seed = parse_u64_option(&args, "--bootstrap-seed", 0x5041_4952_4544_0001)?;
    let sign_flip_draws = parse_usize_option(&args, "--sign-flip-draws", 99_999)?;
    let sign_flip_seed = parse_u64_option(&args, "--sign-flip-seed", 0x5349_474E_464C_4950)?;

    let margin = args
        .get("--equivalence-margin")
        .map(|value| parse_finite(value, "--equivalence-margin"))
        .transpose()?;
    let rationale = args.get("--margin-rationale");
    let input_sha256 = args
        .get("--input-sha256")
        .map(|value| validate_sha256(value))
        .transpose()?;
    match (margin, rationale) {
        (Some(_), None) => return Err(
            "--margin-rationale is required with --equivalence-margin; document the preregistered scientific basis".into()
        ),
        (None, Some(_)) => return Err(
            "--margin-rationale requires --equivalence-margin".into()
        ),
        _ => {}
    }

    let csv = fs::read_to_string(&input_path)?;
    let mut lines = csv.lines();
    let header_line = lines.next().ok_or("input CSV is empty")?;
    let headers = split_unquoted_csv(header_line)?;
    let mut columns = BTreeMap::new();
    for (index, header) in headers.iter().enumerate() {
        if columns.insert(*header, index).is_some() {
            return Err(format!("duplicate CSV column {header}").into());
        }
    }
    let condition_column = *columns.get("condition").ok_or("missing condition column")?;
    let seed_column = *columns.get("seed").ok_or("missing seed column")?;
    let metric_column = *columns
        .get(metric)
        .ok_or_else(|| format!("unknown metric column {metric}"))?;

    let mut control = Vec::new();
    let mut treatment = Vec::new();
    let mut data_rows = 0usize;
    for (line_index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_unquoted_csv(line)
            .map_err(|error| format!("CSV line {}: {error}", line_index + 2))?;
        if fields.len() != headers.len() {
            return Err(format!(
                "CSV line {} has {} fields; expected {}",
                line_index + 2,
                fields.len(),
                headers.len()
            )
            .into());
        }
        data_rows += 1;
        let condition = fields[condition_column];
        if condition != control_name && condition != treatment_name {
            continue;
        }
        let seed_id = fields[seed_column]
            .parse::<u64>()
            .map_err(|_| format!("CSV line {} has invalid seed key", line_index + 2))?;
        let value = parse_finite(fields[metric_column], "metric value")
            .map_err(|error| format!("CSV line {}: {error}", line_index + 2))?;
        let row = SeededValue { seed_id, value };
        if condition == control_name {
            control.push(row);
        } else {
            treatment.push(row);
        }
    }

    let pairs = pair_by_seed(&control, &treatment)?;
    let summary = summarize_paired(&pairs)?;
    let sign_flip = paired_sign_flip_test(
        &pairs,
        SignFlipConfig {
            monte_carlo_draws: sign_flip_draws,
            rng_seed: sign_flip_seed,
        },
    )?;
    let bootstrap = bootstrap_mean_interval(
        &pairs,
        BootstrapConfig {
            replicates: bootstrap_replicates,
            confidence_level: 0.95,
            rng_seed: bootstrap_seed,
        },
    )?;

    let equivalence_json = if let Some(margin) = margin {
        let screen = screen_equivalence(bootstrap, margin)?;
        let status = match screen.status {
            EquivalenceStatus::EquivalentWithinMargin => "equivalent_within_margin",
            EquivalenceStatus::NotEstablished => "not_established",
        };
        format!(
            r#"{{"status":"{status}","margin":{margin:.17e},"rationale":{}}}"#,
            json_string(rationale.expect("validated rationale"))
        )
    } else {
        r#"{"status":"not_evaluated","reason":"no preregistered margin supplied"}"#.to_string()
    };

    let seeds = pairs
        .iter()
        .map(|pair| pair.seed_id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let path_json = json_string(&input_path.to_string_lossy());
    let input_sha256_json = input_sha256.as_deref().map(json_string).unwrap_or_else(|| "null".to_string());
    println!(
        concat!(
            "{{\n",
            "  \"schema\": \"luminous.symtropy.paired-analysis-receipt.v0.1\",\n",
            "  \"status\": \"analysis_only_not_historical_verdict\",\n",
            "  \"input_path\": {},\n",
            "  \"input_sha256\": {},\n",
            "  \"input_data_rows\": {},\n",
            "  \"control\": {},\n",
            "  \"treatment\": {},\n",
            "  \"metric\": {},\n",
            "  \"independent_unit\": \"complete_simulation_seed_replicate\",\n",
            "  \"pairing_key\": \"actual_seed_value\",\n",
            "  \"seed_ids\": [{}],\n",
            "  \"n\": {},\n",
            "  \"paired_summary\": {{\n",
            "    \"mean_difference\": {:.17e},\n",
            "    \"median_difference\": {:.17e},\n",
            "    \"sample_standard_deviation\": {},\n",
            "    \"standard_error\": {},\n",
            "    \"cohens_dz\": {}\n",
            "  }},\n",
            "  \"sign_flip\": {{\n",
            "    \"alternative\": \"two_sided\",\n",
            "    \"p_value\": {:.17e},\n",
            "    \"exact\": {},\n",
            "    \"permutations_evaluated\": {},\n",
            "    \"rng_seed\": {}\n",
            "  }},\n",
            "  \"bootstrap_mean_interval\": {{\n",
            "    \"method\": \"percentile_seed_pair_resampling\",\n",
            "    \"confidence_level\": {:.17e},\n",
            "    \"lower\": {:.17e},\n",
            "    \"upper\": {:.17e},\n",
            "    \"replicates\": {},\n",
            "    \"rng_seed\": {}\n",
            "  }},\n",
            "  \"equivalence_screen\": {}\n",
            "}}"
        ),
        path_json,
        input_sha256_json,
        data_rows,
        json_string(control_name),
        json_string(treatment_name),
        json_string(metric),
        seeds,
        summary.n,
        summary.mean_difference,
        summary.median_difference,
        optional_number(summary.sample_standard_deviation),
        optional_number(summary.standard_error),
        optional_number(summary.cohens_dz),
        sign_flip.p_value,
        sign_flip.exact,
        sign_flip.permutations_evaluated,
        sign_flip
            .rng_seed
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        bootstrap.confidence_level,
        bootstrap.lower,
        bootstrap.upper,
        bootstrap.replicates,
        bootstrap.rng_seed,
        equivalence_json
    );
    Ok(())
}

fn parse_args() -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut parsed = BTreeMap::new();
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--help" {
            println!(
                "Usage: cooperation_paired_analysis --input FILE --control CONDITION --treatment CONDITION --metric COLUMN [--bootstrap-replicates N] [--bootstrap-seed N] [--sign-flip-draws N] [--sign-flip-seed N] [--equivalence-margin X --margin-rationale TEXT]\n\
                 Metrics: alive, energy, cooperation, well_energy, resonance_energy\n\
                 Input: unquoted CSV emitted by cooperation_decomposition."
            );
            std::process::exit(0);
        }
        if !flag.starts_with("--") {
            return Err(format!("unexpected argument {flag}").into());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        if parsed.insert(flag.clone(), value).is_some() {
            return Err(format!("duplicate argument {flag}").into());
        }
    }
    for key in parsed.keys() {
        if !matches!(
            key.as_str(),
            "--input"
                | "--control"
                | "--treatment"
                | "--metric"
                | "--bootstrap-replicates"
                | "--bootstrap-seed"
                | "--sign-flip-draws"
                | "--sign-flip-seed"
                | "--equivalence-margin"
                | "--margin-rationale"
                | "--input-sha256"
        ) {
            return Err(format!("unknown argument {key}").into());
        }
    }
    Ok(parsed)
}

fn required<'a>(args: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, Box<dyn Error>> {
    args.get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required argument {name}").into())
}

fn parse_usize_option(
    args: &BTreeMap<String, String>,
    name: &str,
    default: usize,
) -> Result<usize, Box<dyn Error>> {
    match args.get(name) {
        Some(value) => {
            let parsed = value.parse::<usize>()?;
            if parsed == 0 {
                return Err(format!("{name} must be greater than zero").into());
            }
            Ok(parsed)
        }
        None => Ok(default),
    }
}

fn parse_u64_option(
    args: &BTreeMap<String, String>,
    name: &str,
    default: u64,
) -> Result<u64, Box<dyn Error>> {
    match args.get(name) {
        Some(value) => parse_u64(value, name).map_err(Into::into),
        None => Ok(default),
    }
}

fn parse_u64(value: &str, name: &str) -> Result<u64, String> {
    if let Some(hex) = value.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| format!("invalid {name}: {value}"))
    } else {
        value
            .parse::<u64>()
            .map_err(|_| format!("invalid {name}: {value}"))
    }
}

fn validate_sha256(value: &str) -> Result<String, String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("--input-sha256 must be exactly 64 hexadecimal characters".into());
    }
    Ok(value.to_ascii_lowercase())
}

fn parse_finite(value: &str, name: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("invalid {name}: {value}"))?;
    if !parsed.is_finite() {
        return Err(format!("{name} must be finite"));
    }
    Ok(parsed)
}

fn split_unquoted_csv(line: &str) -> Result<Vec<&str>, String> {
    if line.contains('"') {
        return Err("quoted CSV cells are not supported by this experiment-output reader".into());
    }
    Ok(line.split(',').map(str::trim).collect())
}

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn optional_number(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.17e}"))
        .unwrap_or_else(|| "null".to_string())
}
