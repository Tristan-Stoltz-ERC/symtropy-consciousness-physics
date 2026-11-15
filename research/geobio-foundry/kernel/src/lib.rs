#![forbid(unsafe_code)]
//! Small, dependency-free reference kernel for GeoBio Foundry.
//!
//! This crate deliberately handles only typed material/energy accounting and
//! evidence validation. It is not a bioprocess model, pressure-vessel solver,
//! geomechanics solver, or certification tool.

use std::collections::BTreeSet;

/// The epistemic status of a process claim. These states are deliberately
/// not interchangeable: a modelled result is not a measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceState {
    Assumed,
    Modelled,
    Measured,
    IndependentlyReproduced,
    Qualified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginClass {
    LocalMeasured,
    LocalInferred,
    Imported,
    Recycled,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementMass {
    /// Element symbol or stable material ontology identifier, e.g. "Fe".
    pub element: String,
    /// Elemental mass in kilograms, not compound mass.
    pub mass_kg: f64,
    /// One-standard-deviation uncertainty in kilograms.
    pub standard_uncertainty_kg: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialFlow {
    pub material_id: String,
    pub origin: OriginClass,
    /// True only when the elemental assay covers the complete composition
    /// for this flow. If false, an unlisted element is unknown, not zero.
    pub composition_complete: bool,
    pub elements: Vec<ElementMass>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    /// SHA-256 of frozen input data, lowercase or uppercase hexadecimal.
    pub input_digest: String,
    /// Optional SHA-256 of the model/configuration used.
    pub model_digest: Option<String>,
    pub method_id: String,
    /// Required for independently reproduced and qualified claims.
    pub independent_witness: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceProcess {
    pub process_id: String,
    /// Explicit functional unit, e.g. "1 kg on-spec oxygen at outlet".
    pub functional_unit: String,
    pub evidence_state: EvidenceState,
    pub inputs: Vec<MaterialFlow>,
    pub products: Vec<MaterialFlow>,
    pub waste_streams: Vec<MaterialFlow>,
    /// Gross process energy consumed, in joules.
    pub energy_consumed_j: f64,
    /// Separately reported recovered energy, in joules. It is not silently
    /// netted against gross consumption because recovery quality and timing
    /// may differ.
    pub energy_recovered_j: Option<f64>,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BalanceStatus {
    WithinTolerance,
    OutsideTolerance,
    Unquantified,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementBalance {
    /// Positive means elemental mass is unaccounted for in products+waste.
    pub residual_kg: f64,
    pub element: String,
    /// RSS combination of reported standard uncertainties, assuming
    /// independence. None means at least one relevant uncertainty is absent
    /// or an unassayed flow could contain this element.
    pub combined_standard_uncertainty_kg: Option<f64>,
    pub status: BalanceStatus,
}

/// Process-level result. Per-element checks may pass while the overall
/// inventory remains unquantified if any stream has incomplete composition.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementalBalanceReport {
    pub element_balances: Vec<ElementBalance>,
    pub inventory_complete: bool,
    pub overall_status: BalanceStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KernelError {
    InvalidTolerance,
    InvalidZScore,
    InvalidProcess(Vec<String>),
    NumericalOverflow(String),
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTolerance => write!(f, "absolute tolerance must be finite and >= 0"),
            Self::InvalidZScore => write!(f, "z-score must be finite and >= 0"),
            Self::InvalidProcess(errors) => {
                write!(f, "invalid process: {}", errors.join("; "))
            }
            Self::NumericalOverflow(context) => {
                write!(f, "numerical overflow while calculating {context}")
            }
        }
    }
}

impl std::error::Error for KernelError {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn valid_optional_nonnegative(value: Option<f64>) -> bool {
    match value {
        Some(v) => v.is_finite() && v >= 0.0,
        None => true,
    }
}

/// Validate units-by-construction quantities, identities, composition records,
/// and evidence provenance. The kernel uses kg for elemental mass and J for
/// energy; callers must convert source measurements before constructing data.
pub fn validate_process(process: &ResourceProcess) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if !nonempty(&process.process_id) {
        errors.push("process_id must not be empty".to_string());
    }
    if !nonempty(&process.functional_unit) {
        errors.push("functional_unit must not be empty".to_string());
    }
    if process.inputs.is_empty() {
        errors.push("at least one input flow is required".to_string());
    }
    if process.products.is_empty() {
        errors.push("at least one product flow is required".to_string());
    }
    if !process.energy_consumed_j.is_finite() || process.energy_consumed_j < 0.0 {
        errors.push("energy_consumed_j must be finite and >= 0".to_string());
    }
    if !valid_optional_nonnegative(process.energy_recovered_j) {
        errors.push("energy_recovered_j must be finite and >= 0 when present".to_string());
    }

    if !nonempty(&process.provenance.method_id) {
        errors.push("provenance.method_id must not be empty".to_string());
    }
    if !valid_digest(&process.provenance.input_digest) {
        errors.push("provenance.input_digest must be a 64-character SHA-256 hex digest".to_string());
    }
    if let Some(digest) = &process.provenance.model_digest {
        if !valid_digest(digest) {
            errors.push("provenance.model_digest must be a 64-character SHA-256 hex digest".to_string());
        }
    }
    if matches!(
        process.evidence_state,
        EvidenceState::IndependentlyReproduced | EvidenceState::Qualified
    ) && !process
        .provenance
        .independent_witness
        .as_deref()
        .is_some_and(nonempty)
    {
        errors.push("independent_witness is required for independently reproduced or qualified claims".to_string());
    }

    for (group_name, flows) in [
        ("inputs", &process.inputs),
        ("products", &process.products),
        ("waste_streams", &process.waste_streams),
    ] {
        for (flow_index, flow) in flows.iter().enumerate() {
            let prefix = format!("{group_name}[{flow_index}]");
            if !nonempty(&flow.material_id) {
                errors.push(format!("{prefix}.material_id must not be empty"));
            }
            if flow.elements.is_empty() {
                errors.push(format!("{prefix}.elements must not be empty"));
            }
            let mut seen = BTreeSet::new();
            for element in &flow.elements {
                if !nonempty(&element.element) {
                    errors.push(format!("{prefix} has an empty element identifier"));
                }
                if !seen.insert(element.element.as_str()) {
                    errors.push(format!("{prefix} contains duplicate element {}", element.element));
                }
                if !element.mass_kg.is_finite() || element.mass_kg < 0.0 {
                    errors.push(format!("{prefix}.{}.mass_kg must be finite and >= 0", element.element));
                }
                if !valid_optional_nonnegative(element.standard_uncertainty_kg) {
                    errors.push(format!("{prefix}.{}.standard_uncertainty_kg must be finite and >= 0", element.element));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn flow_element(
    flow: &MaterialFlow,
    element: &str,
) -> (f64, f64, bool, bool) {
    if let Some(entry) = flow.elements.iter().find(|entry| entry.element == element) {
        match entry.standard_uncertainty_kg {
            Some(sigma) => (entry.mass_kg, sigma * sigma, true, false),
            None => (entry.mass_kg, 0.0, false, false),
        }
    } else if flow.composition_complete {
        (0.0, 0.0, true, false)
    } else {
        (0.0, 0.0, false, true)
    }
}

/// Calculate elemental mass residuals for all observed elements.
///
/// A result is classified as WithinTolerance only when every contributing
/// flow either quantifies the element or has a complete composition assay,
/// and all relevant standard uncertainties are supplied. Combined uncertainty
/// uses root-sum-square and therefore assumes independent measurement errors.
/// Correlated errors require a richer covariance model and are outside v0.1.
pub fn elemental_balance(
    process: &ResourceProcess,
    absolute_tolerance_kg: f64,
    z_score: f64,
) -> Result<ElementalBalanceReport, KernelError> {
    if !absolute_tolerance_kg.is_finite() || absolute_tolerance_kg < 0.0 {
        return Err(KernelError::InvalidTolerance);
    }
    if !z_score.is_finite() || z_score < 0.0 {
        return Err(KernelError::InvalidZScore);
    }
    validate_process(process).map_err(KernelError::InvalidProcess)?;

    let mut elements = BTreeSet::new();
    for flow in process
        .inputs
        .iter()
        .chain(process.products.iter())
        .chain(process.waste_streams.iter())
    {
        for entry in &flow.elements {
            elements.insert(entry.element.clone());
        }
    }

    let mut balances = Vec::with_capacity(elements.len());
    for element in elements {
        let mut input_mass = 0.0;
        let mut output_mass = 0.0;
        let mut variance = 0.0;
        let mut uncertainty_complete = true;
        let mut composition_complete = true;

        for flow in &process.inputs {
            let (mass, var, has_uncertainty, has_unknown_composition) =
                flow_element(flow, &element);
            input_mass += mass;
            variance += var;
            uncertainty_complete &= has_uncertainty;
            composition_complete &= !has_unknown_composition;
        }
        for flow in process.products.iter().chain(process.waste_streams.iter()) {
            let (mass, var, has_uncertainty, has_unknown_composition) =
                flow_element(flow, &element);
            output_mass += mass;
            variance += var;
            uncertainty_complete &= has_uncertainty;
            composition_complete &= !has_unknown_composition;
        }

        let residual = input_mass - output_mass;
        if !input_mass.is_finite()
            || !output_mass.is_finite()
            || !variance.is_finite()
            || !residual.is_finite()
        {
            return Err(KernelError::NumericalOverflow(format!("elemental balance for {element}")));
        }
        let sigma = if uncertainty_complete && composition_complete {
            Some(variance.sqrt())
        } else {
            None
        };
        let status = match sigma {
            Some(s) => {
                let threshold = absolute_tolerance_kg + z_score * s;
                if !threshold.is_finite() {
                    return Err(KernelError::NumericalOverflow(format!("tolerance for {element}")));
                }
                if residual.abs() <= threshold {
                    BalanceStatus::WithinTolerance
                } else {
                    BalanceStatus::OutsideTolerance
                }
            }
            None => BalanceStatus::Unquantified,
        };

        balances.push(ElementBalance {
            element,
            residual_kg: residual,
            combined_standard_uncertainty_kg: sigma,
            status,
        });
    }

    let inventory_complete = process
        .inputs
        .iter()
        .chain(process.products.iter())
        .chain(process.waste_streams.iter())
        .all(|flow| flow.composition_complete);
    let overall_status = if balances
        .iter()
        .any(|entry| entry.status == BalanceStatus::OutsideTolerance)
    {
        BalanceStatus::OutsideTolerance
    } else if !inventory_complete
        || balances
            .iter()
            .any(|entry| entry.status == BalanceStatus::Unquantified)
    {
        BalanceStatus::Unquantified
    } else {
        BalanceStatus::WithinTolerance
    };

    Ok(ElementalBalanceReport {
        element_balances: balances,
        inventory_complete,
        overall_status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(symbol: &str, mass: f64) -> ElementMass {
        ElementMass {
            element: symbol.to_string(),
            mass_kg: mass,
            standard_uncertainty_kg: Some(0.0001),
        }
    }

    fn flow(id: &str, elements: Vec<ElementMass>) -> MaterialFlow {
        MaterialFlow {
            material_id: id.to_string(),
            origin: OriginClass::LocalMeasured,
            composition_complete: true,
            elements,
        }
    }

    fn water_electrolysis_fixture() -> ResourceProcess {
        ResourceProcess {
            process_id: "water-electrolysis-reference".to_string(),
            functional_unit: "1 kg water feed processed".to_string(),
            evidence_state: EvidenceState::Modelled,
            inputs: vec![flow(
                "water-feed",
                vec![element("H", 0.111898), element("O", 0.888102)],
            )],
            products: vec![
                flow("hydrogen-product", vec![element("H", 0.111898)]),
                flow("oxygen-product", vec![element("O", 0.888102)]),
            ],
            waste_streams: vec![],
            energy_consumed_j: 1_000.0,
            energy_recovered_j: Some(0.0),
            provenance: Provenance {
                input_digest: "a".repeat(64),
                model_digest: Some("b".repeat(64)),
                method_id: "synthetic-stoichiometric-fixture-v0".to_string(),
                independent_witness: None,
            },
        }
    }

    #[test]
    fn stoichiometric_fixture_balances_each_element() {
        let result = elemental_balance(&water_electrolysis_fixture(), 1e-9, 3.0).unwrap();
        assert_eq!(result.element_balances.len(), 2);
        assert_eq!(result.overall_status, BalanceStatus::WithinTolerance);
        assert!(result.element_balances.iter().all(|entry| entry.status == BalanceStatus::WithinTolerance));
        assert!(result.element_balances.iter().all(|entry| entry.residual_kg.abs() < 1e-12));
    }

    #[test]
    fn missing_product_mass_is_detected() {
        let mut process = water_electrolysis_fixture();
        process.products[1].elements[0].mass_kg = 0.80;
        let result = elemental_balance(&process, 1e-6, 3.0).unwrap();
        let oxygen = result.element_balances.iter().find(|entry| entry.element == "O").unwrap();
        assert_eq!(oxygen.status, BalanceStatus::OutsideTolerance);
        assert!(oxygen.residual_kg > 0.08);
    }

    #[test]
    fn incomplete_inventory_cannot_pass_when_known_elements_balance() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].composition_complete = false;
        let result = elemental_balance(&process, 1e-6, 3.0).unwrap();
        assert_eq!(result.overall_status, BalanceStatus::Unquantified);
        assert!(!result.inventory_complete);
        assert!(result.element_balances.iter().all(|entry| entry.status == BalanceStatus::WithinTolerance));
    }

    #[test]
    fn absent_element_in_incomplete_flow_is_unquantified() {
        let mut process = water_electrolysis_fixture();
        process.products[0].composition_complete = false;
        let result = elemental_balance(&process, 1e-6, 3.0).unwrap();
        let oxygen = result.element_balances.iter().find(|entry| entry.element == "O").unwrap();
        assert_eq!(oxygen.status, BalanceStatus::Unquantified);
        let hydrogen = result.element_balances.iter().find(|entry| entry.element == "H").unwrap();
        assert_eq!(hydrogen.status, BalanceStatus::WithinTolerance);
    }

    #[test]
    fn absent_uncertainty_prevents_qualification() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].elements[0].standard_uncertainty_kg = None;
        let result = elemental_balance(&process, 1e-6, 3.0).unwrap();
        let hydrogen = result.element_balances.iter().find(|entry| entry.element == "H").unwrap();
        assert_eq!(hydrogen.status, BalanceStatus::Unquantified);
    }

    #[test]
    fn negative_mass_is_rejected() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].elements[0].mass_kg = -0.1;
        assert!(validate_process(&process).is_err());
    }

    #[test]
    fn non_finite_element_mass_is_rejected() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].elements[0].mass_kg = f64::NAN;
        assert!(validate_process(&process).is_err());
    }

    #[test]
    fn aggregate_mass_overflow_is_rejected() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].elements[0].mass_kg = 1.0e308;
        process.inputs.push(flow("second-large-feed", vec![element("H", 1.0e308)]));
        let result = elemental_balance(&process, 1e-6, 3.0);
        assert!(matches!(result, Err(KernelError::NumericalOverflow(_))));
    }

    #[test]
    fn qualified_claim_requires_independent_witness() {
        let mut process = water_electrolysis_fixture();
        process.evidence_state = EvidenceState::Qualified;
        assert!(validate_process(&process).is_err());
        process.provenance.independent_witness = Some("external-lab-record-001".to_string());
        assert!(validate_process(&process).is_ok());
    }

    #[test]
    fn invalid_digest_is_rejected() {
        let mut process = water_electrolysis_fixture();
        process.provenance.input_digest = "not-a-digest".to_string();
        assert!(validate_process(&process).is_err());
    }
}
