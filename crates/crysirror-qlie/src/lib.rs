//! QLIE's native .s language. Resources, composition, text, animation and
//! playback belong to CrySirror services; this crate owns script semantics.
//! Format references: Habr https://habr.com/en/articles/426431/ and
//! VNTranslationTools QlieScript.cs (encoding, command and dialogue syntax).
#![forbid(unsafe_code)]
#[cfg(feature = "runtime")]
pub mod adapter;
pub mod expression;
pub mod machine;
pub mod program;
