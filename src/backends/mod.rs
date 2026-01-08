//! Backend implementations.

#[cfg(feature = "mock")]
pub mod mock;

#[cfg(feature = "pass")]
pub mod pass;

#[cfg(feature = "bitwarden")]
pub mod bitwarden;

#[cfg(feature = "aws")]
pub mod aws;

#[cfg(feature = "onepassword")]
pub mod onepassword;

#[cfg(feature = "gcp")]
pub mod gcp;

// TODO: Add remaining backends
//
// #[cfg(all(feature = "wincred", target_os = "windows"))]
// pub mod wincred;
//
// #[cfg(feature = "azure")]
// pub mod azure;

/// Registers all compiled backends with the factory.
///
/// This should be called automatically when the library is used,
/// but can also be called explicitly if needed.
pub fn register_all() {
    #[cfg(feature = "mock")]
    mock::register();

    #[cfg(feature = "pass")]
    pass::register();

    #[cfg(feature = "bitwarden")]
    bitwarden::register();

    #[cfg(feature = "aws")]
    aws::register();

    #[cfg(feature = "onepassword")]
    onepassword::register();

    #[cfg(feature = "gcp")]
    gcp::register();

    // TODO: Register other backends as they're implemented
}
