#[cfg(all(tare_sdk, not(tare_external_entry)))]
mod sdk;
#[cfg(all(tare_sdk, not(tare_external_entry)))]
pub use sdk::*;
#[cfg(all(tare_sdk, tare_external_entry))]
mod sdk_external;
#[cfg(all(tare_sdk, tare_external_entry))]
pub use sdk_external::*;
#[cfg(not(tare_sdk))]
mod raw;
#[cfg(not(tare_sdk))]
pub use raw::*;
