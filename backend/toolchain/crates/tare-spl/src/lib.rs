//! SPL account adapters and CPI wrappers over Tare account views.
mod program;
#[cfg(feature = "associated_token")]
pub mod associated_token;
#[cfg(feature = "mint")]
pub mod mint;
#[cfg(feature = "token")]
pub mod token;
#[cfg(feature = "token_2022")]
pub mod token_2022;
#[cfg(feature = "token_2022")]
pub mod token_interface;
#[cfg(feature = "token_2022_extensions")]
pub mod token_2022_extensions;
#[cfg(feature = "metadata")]
pub mod metadata;
#[cfg(feature = "memo")]
pub mod memo;
#[cfg(feature = "governance")]
compile_error!("Tare does not implement anchor-spl governance");
#[cfg(feature = "stake")]
pub mod stake;
