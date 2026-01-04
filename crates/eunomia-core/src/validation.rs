//! Validation traits and types for Eunomia core types.
//!
//! This module provides a consistent validation framework used across
//! all core types to ensure data integrity and constraint satisfaction.

use std::fmt;

/// Errors that can occur during validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// The field or path that failed validation.
    pub field: String,
    /// A human-readable description of the validation failure.
    pub message: String,
    /// The kind of validation that failed.
    pub kind: ValidationErrorKind,
}

impl ValidationError {
    /// Creates a new validation error.
    ///
    /// # Arguments
    ///
    /// * `field` - The field path that failed validation (e.g., `caller.spiffe_id`)
    /// * `message` - Human-readable error description
    /// * `kind` - The category of validation failure
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_core::ValidationError;
    /// use eunomia_core::validation::ValidationErrorKind;
    ///
    /// let error = ValidationError::new(
    ///     "caller.spiffe_id",
    ///     "SPIFFE ID must start with 'spiffe://'",
    ///     ValidationErrorKind::Format,
    /// );
    /// ```
    pub fn new(field: impl Into<String>, message: impl Into<String>, kind: ValidationErrorKind) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            kind,
        }
    }

    /// Creates a validation error for a required field that is missing.
    pub fn required(field: impl Into<String>) -> Self {
        let field = field.into();
        Self {
            message: format!("'{field}' is required but was not provided"),
            field,
            kind: ValidationErrorKind::Required,
        }
    }

    /// Creates a validation error for an invalid format.
    pub fn format(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            kind: ValidationErrorKind::Format,
        }
    }

    /// Creates a validation error for a value out of range.
    pub fn range(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            kind: ValidationErrorKind::Range,
        }
    }

    /// Creates a validation error for an empty collection.
    pub fn empty(field: impl Into<String>) -> Self {
        let field = field.into();
        Self {
            message: format!("'{field}' must not be empty"),
            field,
            kind: ValidationErrorKind::Empty,
        }
    }

    /// Creates a validation error for a constraint violation.
    pub fn constraint(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            kind: ValidationErrorKind::Constraint,
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "validation error for '{}': {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// The category of validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationErrorKind {
    /// A required field was not provided.
    Required,
    /// The value format is invalid.
    Format,
    /// The value is outside the allowed range.
    Range,
    /// A collection is empty when it shouldn't be.
    Empty,
    /// A business constraint was violated.
    Constraint,
    /// Multiple validation errors occurred.
    Multiple,
}

impl fmt::Display for ValidationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Required => write!(f, "required"),
            Self::Format => write!(f, "format"),
            Self::Range => write!(f, "range"),
            Self::Empty => write!(f, "empty"),
            Self::Constraint => write!(f, "constraint"),
            Self::Multiple => write!(f, "multiple"),
        }
    }
}

/// A collection of validation errors.
#[derive(Debug, Clone, Default)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl ValidationErrors {
    /// Creates an empty validation errors collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a validation error to the collection.
    pub fn add(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Returns true if there are no validation errors.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns the number of validation errors.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.errors.len()
    }

    /// Returns an iterator over the validation errors.
    pub fn iter(&self) -> impl Iterator<Item = &ValidationError> {
        self.errors.iter()
    }

    /// Converts to a Result, returning `Ok(())` if no errors, or `Err` with the first error.
    ///
    /// # Errors
    ///
    /// Returns the first `ValidationError` if any errors exist in the collection.
    pub fn into_result(self) -> Result<(), ValidationError> {
        self.errors.into_iter().next().map_or(Ok(()), Err)
    }

    /// Merges another `ValidationErrors` into this one.
    pub fn merge(&mut self, other: Self) {
        self.errors.extend(other.errors);
    }
}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl FromIterator<ValidationError> for ValidationErrors {
    fn from_iter<T: IntoIterator<Item = ValidationError>>(iter: T) -> Self {
        Self {
            errors: iter.into_iter().collect(),
        }
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errors.is_empty() {
            write!(f, "no validation errors")
        } else if self.errors.len() == 1 {
            write!(f, "{}", self.errors[0])
        } else {
            writeln!(f, "{} validation errors:", self.errors.len())?;
            for error in &self.errors {
                writeln!(f, "  - {error}")?;
            }
            Ok(())
        }
    }
}

impl std::error::Error for ValidationErrors {}

/// Trait for types that can be validated.
///
/// Implementing this trait allows a type to participate in the validation
/// framework and report any constraint violations.
///
/// # Examples
///
/// ```
/// use eunomia_core::validation::{Validate, ValidationError, ValidationErrors};
///
/// struct Email(String);
///
/// impl Validate for Email {
///     fn validate(&self) -> Result<(), ValidationErrors> {
///         let mut errors = ValidationErrors::new();
///         
///         if !self.0.contains('@') {
///             errors.add(ValidationError::format(
///                 "email",
///                 "must contain '@' symbol"
///             ));
///         }
///         
///         if self.0.is_empty() {
///             errors.add(ValidationError::empty("email"));
///         }
///         
///         if errors.is_empty() {
///             Ok(())
///         } else {
///             Err(errors)
///         }
///     }
/// }
/// ```
pub trait Validate {
    /// Validates this instance and returns any errors found.
    ///
    /// Returns `Ok(())` if validation passes, or `Err(ValidationErrors)`
    /// containing all validation failures.
    ///
    /// # Errors
    ///
    /// Returns `ValidationErrors` containing all validation failures found.
    fn validate(&self) -> Result<(), ValidationErrors>;

    /// Returns true if this instance is valid.
    ///
    /// This is a convenience method that calls `validate()` and checks
    /// if it returned `Ok(())`.
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_new() {
        let error = ValidationError::new("field", "message", ValidationErrorKind::Required);
        assert_eq!(error.field, "field");
        assert_eq!(error.message, "message");
        assert_eq!(error.kind, ValidationErrorKind::Required);
    }

    #[test]
    fn test_validation_error_required() {
        let error = ValidationError::required("username");
        assert_eq!(error.field, "username");
        assert_eq!(error.kind, ValidationErrorKind::Required);
        assert!(error.message.contains("required"));
    }

    #[test]
    fn test_validation_error_format() {
        let error = ValidationError::format("email", "must contain @");
        assert_eq!(error.field, "email");
        assert_eq!(error.kind, ValidationErrorKind::Format);
        assert_eq!(error.message, "must contain @");
    }

    #[test]
    fn test_validation_error_range() {
        let error = ValidationError::range("age", "must be between 0 and 150");
        assert_eq!(error.field, "age");
        assert_eq!(error.kind, ValidationErrorKind::Range);
    }

    #[test]
    fn test_validation_error_empty() {
        let error = ValidationError::empty("items");
        assert_eq!(error.field, "items");
        assert_eq!(error.kind, ValidationErrorKind::Empty);
        assert!(error.message.contains("empty"));
    }

    #[test]
    fn test_validation_error_constraint() {
        let error = ValidationError::constraint("password", "must contain special character");
        assert_eq!(error.field, "password");
        assert_eq!(error.kind, ValidationErrorKind::Constraint);
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::format("email", "invalid format");
        let display = format!("{error}");
        assert!(display.contains("email"));
        assert!(display.contains("invalid format"));
    }

    #[test]
    fn test_validation_errors_empty() {
        let errors = ValidationErrors::new();
        assert!(errors.is_empty());
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validation_errors_add() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError::required("field1"));
        errors.add(ValidationError::format("field2", "bad format"));
        
        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_validation_errors_into_result_ok() {
        let errors = ValidationErrors::new();
        assert!(errors.into_result().is_ok());
    }

    #[test]
    fn test_validation_errors_into_result_err() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError::required("field"));
        
        let result = errors.into_result();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field, "field");
    }

    #[test]
    fn test_validation_errors_merge() {
        let mut errors1 = ValidationErrors::new();
        errors1.add(ValidationError::required("field1"));
        
        let mut errors2 = ValidationErrors::new();
        errors2.add(ValidationError::format("field2", "bad"));
        
        errors1.merge(errors2);
        assert_eq!(errors1.len(), 2);
    }

    #[test]
    fn test_validation_errors_iter() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError::required("a"));
        errors.add(ValidationError::required("b"));
        
        let fields: Vec<_> = errors.iter().map(|e| e.field.as_str()).collect();
        assert_eq!(fields, vec!["a", "b"]);
    }

    #[test]
    fn test_validation_errors_from_iterator() {
        let error_vec = vec![
            ValidationError::required("a"),
            ValidationError::required("b"),
        ];
        
        let errors: ValidationErrors = error_vec.into_iter().collect();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_validation_errors_display_empty() {
        let errors = ValidationErrors::new();
        let display = format!("{errors}");
        assert!(display.contains("no validation errors"));
    }

    #[test]
    fn test_validation_errors_display_single() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError::required("field"));
        
        let display = format!("{errors}");
        assert!(display.contains("field"));
        assert!(!display.contains("validation errors:"));
    }

    #[test]
    fn test_validation_errors_display_multiple() {
        let mut errors = ValidationErrors::new();
        errors.add(ValidationError::required("field1"));
        errors.add(ValidationError::format("field2", "bad"));
        
        let display = format!("{errors}");
        assert!(display.contains("2 validation errors"));
        assert!(display.contains("field1"));
        assert!(display.contains("field2"));
    }

    #[test]
    fn test_validation_error_kind_display() {
        assert_eq!(format!("{}", ValidationErrorKind::Required), "required");
        assert_eq!(format!("{}", ValidationErrorKind::Format), "format");
        assert_eq!(format!("{}", ValidationErrorKind::Range), "range");
        assert_eq!(format!("{}", ValidationErrorKind::Empty), "empty");
        assert_eq!(format!("{}", ValidationErrorKind::Constraint), "constraint");
        assert_eq!(format!("{}", ValidationErrorKind::Multiple), "multiple");
    }

    // Test the Validate trait with a simple example
    struct TestStruct {
        value: i32,
    }

    impl Validate for TestStruct {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            
            if self.value < 0 {
                errors.add(ValidationError::range("value", "must be non-negative"));
            }
            
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }

    #[test]
    fn test_validate_trait_valid() {
        let valid = TestStruct { value: 42 };
        assert!(valid.is_valid());
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_validate_trait_invalid() {
        let invalid = TestStruct { value: -1 };
        assert!(!invalid.is_valid());
        
        let result = invalid.validate();
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
    }
}
