//! # tri-canon
//!
//! Name validation and canonicalization with tripwires #97-108.
//!
//! This crate enforces naming conventions and validates service/experiment names
//! across the IGLA project ecosystem.

use regex::Regex;
use std::sync::OnceLock;

/// Canonical name validation result.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Name is valid.
    Valid,
    /// Name is invalid with reason.
    Invalid(String),
}

/// Tripwire ID as specified in the project documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum TripwireId {
    /// Tripwire #97: Empty name
    T97_EmptyName,
    /// Tripwire #98: Name too long
    T98_NameTooLong,
    /// Tripwire #99: Invalid characters
    T99_InvalidCharacters,
    /// Tripwire #100: Reserved prefix
    T100_ReservedPrefix,
    /// Tripwire #101: Duplicate name
    T101_DuplicateName,
    /// Tripwire #102: Invalid seed format
    T102_InvalidSeedFormat,
    /// Tripwire #103: Seed out of range
    T103_SeedOutOfRange,
    /// Tripwire #104: Missing required prefix
    T104_MissingPrefix,
    /// Tripwire #105: Invalid environment suffix
    T105_InvalidEnvSuffix,
    /// Tripwire #106: Consecutive hyphens
    T106_ConsecutiveHyphens,
    /// Tripwire #107: Trailing/leading hyphens
    T107_EdgeHyphens,
    /// Tripwire #108: Disallowed words
    T108_DisallowedWords,
}

/// Tripwire violation with context.
#[derive(Debug, Clone)]
pub struct TripwireViolation {
    /// The tripwire that was triggered.
    pub tripwire: TripwireId,
    /// Human-readable explanation.
    pub message: String,
}

/// Maximum allowed length for a name.
const MAX_NAME_LENGTH: usize = 64;

/// Valid seed range for training experiments.
const VALID_SEED_RANGE: std::ops::RangeInclusive<i32> = 1..=9999;

/// Reserved prefixes that cannot be used.
const RESERVED_PREFIXES: &[&str] = &["sys-", "admin-", "internal-", "test-", "temp-"];

/// Disallowed words in names.
const DISALLOWED_WORDS: &[&str] = &[
    "delete", "drop", "truncate", "destroy", "kill", "nuke", "erase", "remove",
];

/// Regex for valid name characters (lowercase letters, numbers, hyphens).
fn valid_char_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[a-z0-9-]+$").unwrap())
}

/// Regex for seed extraction (e.g., "trios-train-seed-42").
fn seed_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"seed-(\d+)$").unwrap())
}

/// Validate a name for general use.
///
/// # Arguments
///
/// * `name` - The name to validate
///
/// # Returns
///
/// Returns `ValidationResult` indicating validity.
pub fn validate(name: &str) -> ValidationResult {
    let violations = validate_with_tripwires(name);
    if violations.is_empty() {
        ValidationResult::Valid
    } else {
        ValidationResult::Invalid(violations[0].message.clone())
    }
}

/// Validate a name specifically for deployment.
///
/// This is a stricter validation that includes deployment-specific checks.
///
/// # Arguments
///
/// * `name` - The name to validate
///
/// # Returns
///
/// Returns `ValidationResult` indicating validity.
pub fn validate_for_deploy(name: &str) -> ValidationResult {
    let violations = validate_with_tripwires(name);

    // Additional deployment-specific checks
    let deploy_violations: Vec<TripwireViolation> = violations
        .into_iter()
        .chain(check_deploy_specific_rules(name))
        .collect();

    if deploy_violations.is_empty() {
        ValidationResult::Valid
    } else {
        ValidationResult::Invalid(deploy_violations[0].message.clone())
    }
}

/// Validate a name and return all tripwire violations.
///
/// # Arguments
///
/// * `name` - The name to validate
///
/// # Returns
///
/// Returns a vector of all tripwire violations found.
pub fn validate_with_tripwires(name: &str) -> Vec<TripwireViolation> {
    let mut violations = Vec::new();

    // Tripwire #97: Empty name
    if name.is_empty() {
        violations.push(TripwireViolation {
            tripwire: TripwireId::T97_EmptyName,
            message: "Name cannot be empty".to_string(),
        });
        return violations; // No point checking further
    }

    // Tripwire #98: Name too long
    if name.len() > MAX_NAME_LENGTH {
        violations.push(TripwireViolation {
            tripwire: TripwireId::T98_NameTooLong,
            message: format!("Name exceeds maximum length of {}", MAX_NAME_LENGTH),
        });
    }

    // Tripwire #107: Edge hyphens
    if name.starts_with('-') || name.ends_with('-') {
        violations.push(TripwireViolation {
            tripwire: TripwireId::T107_EdgeHyphens,
            message: "Name cannot start or end with a hyphen".to_string(),
        });
    }

    // Tripwire #106: Consecutive hyphens
    if name.contains("--") {
        violations.push(TripwireViolation {
            tripwire: TripwireId::T106_ConsecutiveHyphens,
            message: "Name cannot contain consecutive hyphens".to_string(),
        });
    }

    // Tripwire #99: Invalid characters
    if !valid_char_regex().is_match(name) {
        violations.push(TripwireViolation {
            tripwire: TripwireId::T99_InvalidCharacters,
            message: "Name can only contain lowercase letters, numbers, and hyphens".to_string(),
        });
    }

    // Tripwire #100: Reserved prefix
    for prefix in RESERVED_PREFIXES {
        if name.starts_with(prefix) {
            violations.push(TripwireViolation {
                tripwire: TripwireId::T100_ReservedPrefix,
                message: format!("Name cannot use reserved prefix '{}'", prefix.trim_end_matches('-')),
            });
        }
    }

    // Tripwire #108: Disallowed words
    let lower_name = name.to_lowercase();
    for word in DISALLOWED_WORDS {
        if lower_name.contains(word) {
            violations.push(TripwireViolation {
                tripwire: TripwireId::T108_DisallowedWords,
                message: format!("Name contains disallowed word '{}'", word),
            });
        }
    }

    // Tripwire #102: Invalid seed format (if seed pattern is present)
    if let Some(captures) = seed_regex().captures(name) {
        if let Some(seed_str) = captures.get(1) {
            if let Ok(seed) = seed_str.as_str().parse::<i32>() {
                // Tripwire #103: Seed out of range
                if !VALID_SEED_RANGE.contains(&seed) {
                    violations.push(TripwireViolation {
                        tripwire: TripwireId::T103_SeedOutOfRange,
                        message: format!("Seed {} is out of valid range {:?}", seed, VALID_SEED_RANGE),
                    });
                }
            } else {
                violations.push(TripwireViolation {
                    tripwire: TripwireId::T102_InvalidSeedFormat,
                    message: "Seed value is not a valid integer".to_string(),
                });
            }
        }
    }

    violations
}

/// Check deployment-specific naming rules.
///
/// # Arguments
///
/// * `name` - The name to validate
///
/// # Returns
///
/// Returns a vector of deployment-specific violations.
fn check_deploy_specific_rules(name: &str) -> Vec<TripwireViolation> {
    let mut violations = Vec::new();

    // Tripwire #104: Missing required prefix for training services
    if !name.starts_with("trios-") && !name.starts_with("igla-") {
        violations.push(TripwireViolation {
            tripwire: TripwireId::T104_MissingPrefix,
            message: "Training service name must start with 'trios-' or 'igla-'".to_string(),
        });
    }

    // Tripwire #105: Invalid environment suffix check
    if !name.ends_with("-prod") && !name.ends_with("-staging") && !name.ends_with("-dev") {
        // This is a warning, not necessarily a hard error
        tracing::warn!(
            "name '{}' lacks environment suffix (-prod, -staging, -dev)",
            name
        );
    }

    violations
}

/// Extract seed number from a service name if present.
///
/// # Arguments
///
/// * `name` - The service name
///
/// # Returns
///
/// Returns `Some(seed)` if a valid seed is found, `None` otherwise.
pub fn extract_seed(name: &str) -> Option<i32> {
    seed_regex()
        .captures(name)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Canonicalize a name to a standard format.
///
/// # Arguments
///
/// * `name` - The name to canonicalize
///
/// # Returns
///
/// Returns the canonicalized name.
///
/// # Errors
///
/// Returns an error if the name cannot be canonicalized.
pub fn canonicalize(name: &str) -> anyhow::Result<String> {
    let violations = validate_with_tripwires(name);
    if !violations.is_empty() {
        anyhow::bail!("Cannot canonicalize invalid name: {}", violations[0].message);
    }

    // Convert to lowercase
    let canonical = name.to_lowercase();

    // Replace multiple consecutive hyphens with single hyphen
    let canonical = regex::Regex::new(r"-+").unwrap().replace_all(&canonical, "-");

    // Strip leading/trailing hyphens
    let canonical = canonical.trim_matches('-').to_string();

    Ok(canonical)
}
