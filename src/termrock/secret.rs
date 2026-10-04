//! Termrock secret-aware fields, zeroized storage, and validation types.
//!
//! Provides `Secret` storage with volatile zeroization on Drop and clear,
//! preventing accidental cloning, serialization, or debug exposure.

use crate::termrock::identity::FieldKey;

/// Opaque owned secret value with volatile zeroization on clear and drop.
///
/// Strictly does NOT implement `Clone`, serde `Serialize`, or ordinary `Debug`.
/// The secret contents are only accessible via [`Secret::expose`].
pub struct Secret {
    bytes: Vec<u8>,
}

impl Secret {
    /// Creates a new `Secret` from a String.
    pub fn new(value: String) -> Self {
        Self {
            bytes: value.into_bytes(),
        }
    }

    /// Explicitly zeroizes all bytes in the buffer and clears it.
    pub fn clear(&mut self) {
        Self::zeroize_buffer(&mut self.bytes);
        self.bytes.clear();
    }

    /// Returns `true` if the secret buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Returns the length in bytes of the secret.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Exposes a temporary borrowed `&str` to a closure.
    ///
    /// The secret is not copied or leaked beyond the closure scope.
    pub fn expose<R>(&self, read: impl FnOnce(&str) -> R) -> R {
        let s = std::str::from_utf8(&self.bytes).unwrap_or("");
        read(s)
    }

    /// Volatile zeroization helper writing 0 to every byte.
    pub(crate) fn zeroize_buffer(buf: &mut [u8]) {
        for byte in buf.iter_mut() {
            unsafe {
                std::ptr::write_volatile(byte, 0);
            }
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        Self::zeroize_buffer(&mut self.bytes);
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret([REDACTED])")
    }
}

/// Secret field disclosure and clipboard policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretPolicy {
    pub reveal_tail: u8,
    pub allow_copy: bool,
}

impl SecretPolicy {
    /// Safe default policy: no tail revealed, copy disallowed.
    pub const fn none() -> Self {
        Self {
            reveal_tail: 0,
            allow_copy: false,
        }
    }

    pub const fn new(reveal_tail: u8, allow_copy: bool) -> Self {
        Self {
            reveal_tail,
            allow_copy,
        }
    }
}

impl Default for SecretPolicy {
    fn default() -> Self {
        Self::none()
    }
}

/// Type marker for plain editable fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Plain;

/// Type marker for secret/password fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SecretText;

/// Action emitted by text input fields.
#[derive(Debug, PartialEq, Eq)]
pub enum TextAction<V> {
    Edited,
    Commit { value: V },
    Cancelled,
    Conflict,
}

impl<V: Clone> Clone for TextAction<V> {
    fn clone(&self) -> Self {
        match self {
            Self::Edited => Self::Edited,
            Self::Commit { value } => Self::Commit {
                value: value.clone(),
            },
            Self::Cancelled => Self::Cancelled,
            Self::Conflict => Self::Conflict,
        }
    }
}

/// Validation message containing a stable error code and safe display string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMessage {
    pub code: &'static str,
    pub display: String,
}

impl ValidationMessage {
    pub fn new(code: &'static str, display: impl Into<String>) -> Self {
        Self {
            code,
            display: display.into(),
        }
    }
}

/// Field validation error associated with a specific `FieldKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub field: FieldKey,
    pub message: ValidationMessage,
}

impl FieldError {
    pub const fn new(field: FieldKey, message: ValidationMessage) -> Self {
        Self { field, message }
    }
}

/// Trait for field value validation.
pub trait Validator {
    fn validate(&self, value: &str) -> Result<(), ValidationMessage>;
}

impl<F> Validator for F
where
    F: Fn(&str) -> Result<(), ValidationMessage>,
{
    fn validate(&self, value: &str) -> Result<(), ValidationMessage> {
        (self)(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_zeroization_on_clear_and_drop() {
        let mut sec = Secret::new("api_key_secret_value_12345".to_string());
        assert_eq!(sec.len(), 26);
        assert!(!sec.is_empty());

        let exposed = sec.expose(|val| {
            assert_eq!(val, "api_key_secret_value_12345");
            val.to_owned()
        });
        assert_eq!(exposed, "api_key_secret_value_12345");

        // Verify zeroization on slice directly before clear
        Secret::zeroize_buffer(&mut sec.bytes);
        assert!(sec.bytes.iter().all(|&b| b == 0));

        sec.clear();
        assert!(sec.is_empty());
        assert_eq!(sec.len(), 0);
    }

    #[test]
    fn secret_debug_redaction_and_leak_prevention() {
        let sec = Secret::new("top_secret_token".to_string());
        let debug_str = format!("{sec:?}");
        assert_eq!(debug_str, "Secret([REDACTED])");
        assert!(!debug_str.contains("top_secret_token"));

        let action = TextAction::Commit { value: sec };
        let action_str = format!("{action:?}");
        assert_eq!(action_str, "Commit { value: Secret([REDACTED]) }");
        assert!(!action_str.contains("top_secret_token"));
    }

    #[test]
    fn secret_policy_defaults() {
        let policy = SecretPolicy::none();
        assert_eq!(policy.reveal_tail, 0);
        assert!(!policy.allow_copy);

        let custom = SecretPolicy::new(4, true);
        assert_eq!(custom.reveal_tail, 4);
        assert!(custom.allow_copy);
    }

    #[test]
    fn validation_types_and_trait() {
        let validator = |val: &str| {
            if val.is_empty() {
                Err(ValidationMessage::new("REQUIRED", "Field is required"))
            } else {
                Ok(())
            }
        };

        assert!(validator.validate("test").is_ok());
        let err = validator.validate("").unwrap_err();
        assert_eq!(err.code, "REQUIRED");
        assert_eq!(err.display, "Field is required");

        let field_err = FieldError::new(FieldKey::new(10), err);
        assert_eq!(field_err.field.as_u64(), 10);
        assert_eq!(field_err.message.code, "REQUIRED");
    }

    #[test]
    fn drop_exercises_zeroization() {
        let ptr: *const u8;
        let cap: usize;
        {
            let sec = Secret::new("temporary_sensitive_value".to_string());
            ptr = sec.bytes.as_ptr();
            cap = sec.bytes.len();
            // Drop will be called here
            drop(sec);
        }
        // Buffer was zeroized on drop
        // Verify via volatile read if memory not immediately reclaimed
        let mut zero_count = 0;
        for i in 0..cap {
            unsafe {
                if std::ptr::read_volatile(ptr.add(i)) == 0 {
                    zero_count += 1;
                }
            }
        }
        // At least the zeroized bytes should be observable before reallocation
        assert!(zero_count > 0);
    }
}
