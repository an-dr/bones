use crate::{Integration, Locations};

/// Writes every part of the registration. Registering again replaces a stale
/// or partial registration rather than adding a second one.
pub fn register(integration: &Integration, locations: &Locations) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return crate::linux::register(integration, locations);
    #[cfg(windows)]
    return crate::windows::register(integration, locations);
    #[cfg(not(any(target_os = "linux", windows)))]
    return Err(unsupported(integration, locations));
}

/// Removes every part of the registration. Removing something that is not
/// registered succeeds, and the application's own files are left in place.
pub fn unregister(integration: &Integration, locations: &Locations) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return crate::linux::unregister(integration, locations);
    #[cfg(windows)]
    return crate::windows::unregister(integration, locations);
    #[cfg(not(any(target_os = "linux", windows)))]
    return Err(unsupported(integration, locations));
}

/// Whether every part of the registration is present and current. A partial
/// or stale registration reports `false`, so registering again repairs it.
pub fn is_registered(integration: &Integration, locations: &Locations) -> bool {
    #[cfg(target_os = "linux")]
    return crate::linux::is_registered(integration, locations);
    #[cfg(windows)]
    return crate::windows::is_registered(integration, locations);
    #[cfg(not(any(target_os = "linux", windows)))]
    return {
        let _ = (integration, locations);
        false
    };
}

#[cfg(not(any(target_os = "linux", windows)))]
fn unsupported(_integration: &Integration, _locations: &Locations) -> String {
    String::from("desktop registration is not supported on this operating system")
}
