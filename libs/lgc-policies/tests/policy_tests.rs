// Copyright (c) 2023 LogCraft.io.
// SPDX-License-Identifier: MPL-2.0

use lgc_policies::policy::{CheckKind, Constraint, Policy, Severity};
use rstest::rstest;
use serde_json::Value;

/// Helper function that validates a YAML sample against the JSON Schema
/// generated from a given policy.
pub fn validate_sample_yaml(policy: &Policy, sample_yaml: &str) -> bool {
    let instance: Value = serde_yaml_ng::from_str(sample_yaml).expect("Invalid YAML");
    let schema = match policy.to_schema() {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to generate schema for policy: {e}");
            return false;
        },
    };
    
    jsonschema::validate(&schema, &instance).is_ok()
}

/// Policy produces the correct default error message.
#[rstest]
#[case(
    CheckKind::Existence,
    Severity::Error,
    "field '/username' must be present"
)]
#[case(
    CheckKind::Absence,
    Severity::Error,
    "field '/username' must not be present"
)]
#[case(
    CheckKind::Constraint,
    Severity::Error,
    "field '/username' doesn't respect constraint"
)]
#[case(
    CheckKind::Pattern,
    Severity::Error,
    "field '/username' doesn't match pattern"
)]
fn test_default_message(
    #[case] check: CheckKind,
    #[case] severity: Severity,
    #[case] expected: &str,
) {
    let policy = Policy {
        field: "/username".to_string(),
        check,
        severity,
        message: None,
        ignorecase: None,
        regex: None,
        constraints: None,
    };
    assert_eq!(policy.default_message(), expected);
}

/// Pattern Checks
#[rstest]
// Case-insensitive
#[case(r#"title: "MY-123 Title""#, true, true)]
#[case(r#"title: "My-123 title""#, true, true)]
#[case(r#"title: "123-my title""#, true, false)]
// Case-sensitive
#[case(r#"title: "MY-123 Title""#, false, true)]
#[case(r#"title: "My-123 title""#, false, false)]
#[case(r#"title: "123-my title""#, false, false)]
fn test_pattern(#[case] sample: &str, #[case] ignorecase: bool, #[case] expected: bool) {
    let policy = Policy {
        field: "/title".to_string(),
        check: CheckKind::Pattern,
        severity: Severity::Error,
        message: None, // Use default message.
        ignorecase: Some(ignorecase),
        regex: None,
        constraints: None,
    };
    // Missing regex.
    let result = validate_sample_yaml(&policy, sample);
    assert!(!result);

    let policy = Policy {
        field: "/title".to_string(),
        check: CheckKind::Pattern,
        severity: Severity::Error,
        message: None, // Use default message.
        ignorecase: Some(ignorecase),
        regex: Some(r"^[A-Z]+-\d+\s\S+".to_string()),
        constraints: None,
    };
    let result = validate_sample_yaml(&policy, sample);
    assert_eq!(result, expected);
}

/// Existence Checks
#[rstest]
#[case(r#"username: "bob""#, true)]
#[case(r#"other: "data""#, false)]
fn test_existence(#[case] sample: &str, #[case] expected: bool) {
    let policy = Policy {
        field: "/username".to_string(),
        check: CheckKind::Existence,
        severity: Severity::Error,
        message: None,
        ignorecase: Some(false),
        regex: None,
        constraints: None,
    };
    assert_eq!(validate_sample_yaml(&policy, sample), expected);
}

/// Absence Checks
#[rstest]
#[case(r#"username: "alice""#, true)]
#[case(r#"password: "secret""#, false)]
fn test_absence(#[case] sample: &str, #[case] expected: bool) {
    let policy = Policy {
        field: "/password".to_string(),
        check: CheckKind::Absence,
        severity: Severity::Warning,
        message: None,
        ignorecase: Some(false),
        regex: None,
        constraints: None,
    };
    assert_eq!(validate_sample_yaml(&policy, sample), expected);
}

/// Constraint Checks: minLength
#[rstest]
#[case(r#"name: "Michael""#, true)]
#[case(r#"name: "Joe""#, false)]
fn test_constraint_min_length(#[case] sample: &str, #[case] expected: bool) {
    let policy = Policy {
        field: "/name".to_string(),
        check: CheckKind::Constraint,
        severity: Severity::Error,
        message: None,
        ignorecase: Some(false),
        regex: None,
        constraints: None
    };
    // Missing constraints specification.
    let result = validate_sample_yaml(&policy, sample);
    assert!(!result);

    let policy = Policy {
        field: "/name".to_string(),
        check: CheckKind::Constraint,
        severity: Severity::Error,
        message: None,
        ignorecase: Some(false),
        regex: None,
        constraints: Some(Constraint {
            min_length: Some(5),
            max_length: None,
            values: None,
        }),
    };
    assert_eq!(validate_sample_yaml(&policy, sample), expected);
}

/// Constraint Checks: maxLength
#[rstest]
#[case(r#"name: "Joe""#, true)]
#[case(r#"name: "Michael""#, false)]
fn test_constraint_max_length(#[case] sample: &str, #[case] expected: bool) {
    let policy = Policy {
        field: "/name".to_string(),
        check: CheckKind::Constraint,
        severity: Severity::Error,
        message: None,
        ignorecase: Some(false),
        regex: None,
        constraints: Some(Constraint {
            min_length: None,
            max_length: Some(5),
            values: None,
        }),
    };
    assert_eq!(validate_sample_yaml(&policy, sample), expected);
}
/// Constraint Checks: one-of
#[rstest]
// Case-insensitive
#[case(r#"color: "red""#, true, true)]
#[case(r#"color: "ReD""#, true, true)]
#[case(r#"color: "yellow""#, true, false)]
#[case(r#"color: "YELLOW""#, true, false)]
// Case-sensitive
#[case(r#"color: "red""#, false, true)]
#[case(r#"color: "ReD""#, false, false)]
#[case(r#"color: "yellow""#, false, false)]
#[case(r#"color: "YELLOW""#, false, false)]
fn test_constraint_one_of(#[case] sample: &str, #[case] ignorecase: bool, #[case] expected: bool) {
    let policy = Policy {
        field: "/color".to_string(),
        check: CheckKind::Constraint,
        severity: Severity::Warning,
        message: None,
        ignorecase: Some(ignorecase),
        regex: None,
        constraints: Some(Constraint {
            min_length: None,
            max_length: None,
            values: Some(vec![
                "red".to_string(),
                "green".to_string(),
                "blue".to_string(),
            ]),
        }),
    };
    assert_eq!(validate_sample_yaml(&policy, sample), expected);
}

/// Nested Field Checks
#[rstest]
#[case(
    r#"user:
  info:
    email: "test@example.com""#,
    true
)]
#[case(
    r#"user:
  info:
    email: "invalid_email""#,
    false
)]
fn test_nested_pattern(#[case] sample: &str, #[case] expected: bool) {
    let policy = Policy {
        field: "/user/info/email".to_string(),
        check: CheckKind::Pattern,
        severity: Severity::Error,
        message: Some("Email format invalid".to_string()),
        ignorecase: Some(false),
        regex: Some(r"^\S+@\S+\.\S+$".to_string()),
        constraints: None,
    };
    assert_eq!(validate_sample_yaml(&policy, sample), expected);
}

/// Dot Notation Field Checks
#[rstest]
#[case(
    r#"user:
  name: "Bob""#,
    true
)]
#[case(
    r#"user:
  age: 30"#,
    false
)]
fn test_dot_notation_existence(#[case] sample: &str, #[case] expected: bool) {
    let policy = Policy {
        field: "user.name".to_string(),
        check: CheckKind::Existence,
        severity: Severity::Error,
        message: Some("User name must be present".to_string()),
        ignorecase: Some(false),
        regex: None,
        constraints: None,
    };
    assert_eq!(validate_sample_yaml(&policy, sample), expected);
}
