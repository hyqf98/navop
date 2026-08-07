use one_core::storage::RemoteDesktopBackendPreference;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RemoteDesktopPresentation {
    Canvas,
    NativeWindows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RemoteDesktopPlatform {
    Windows,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowsNativeRdpAvailability {
    Available,
    UnavailableNotBuilt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RemoteDesktopPresentationError {
    UnavailableNotBuilt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RemoteDesktopPresentationState {
    Created,
    NativeChildCreated,
    Connecting,
    Active,
    Inactive,
    Reconnecting,
    Closing,
    NativeChildDestroyed,
    Released,
}

pub(crate) const fn current_windows_native_rdp_availability() -> WindowsNativeRdpAvailability {
    WindowsNativeRdpAvailability::UnavailableNotBuilt
}

pub(crate) const fn create_remote_desktop_presentation(
    preference: RemoteDesktopBackendPreference,
) -> Result<RemoteDesktopPresentation, RemoteDesktopPresentationError> {
    select_remote_desktop_presentation(
        current_remote_desktop_platform(),
        preference,
        current_windows_native_rdp_availability(),
    )
}

pub(crate) const fn select_remote_desktop_presentation(
    platform: RemoteDesktopPlatform,
    preference: RemoteDesktopBackendPreference,
    native_availability: WindowsNativeRdpAvailability,
) -> Result<RemoteDesktopPresentation, RemoteDesktopPresentationError> {
    if matches!(platform, RemoteDesktopPlatform::Other) {
        return Ok(RemoteDesktopPresentation::Canvas);
    }

    classify_windows_presentation(preference, native_availability)
}

#[cfg(target_os = "windows")]
const fn current_remote_desktop_platform() -> RemoteDesktopPlatform {
    RemoteDesktopPlatform::Windows
}

#[cfg(not(target_os = "windows"))]
const fn current_remote_desktop_platform() -> RemoteDesktopPlatform {
    RemoteDesktopPlatform::Other
}

const fn classify_windows_presentation(
    preference: RemoteDesktopBackendPreference,
    native_availability: WindowsNativeRdpAvailability,
) -> Result<RemoteDesktopPresentation, RemoteDesktopPresentationError> {
    match (preference, native_availability) {
        (RemoteDesktopBackendPreference::Canvas, _) => Ok(RemoteDesktopPresentation::Canvas),
        (
            RemoteDesktopBackendPreference::Auto | RemoteDesktopBackendPreference::WindowsNative,
            WindowsNativeRdpAvailability::Available,
        ) => Ok(RemoteDesktopPresentation::NativeWindows),
        (
            RemoteDesktopBackendPreference::Auto,
            WindowsNativeRdpAvailability::UnavailableNotBuilt,
        ) => Ok(RemoteDesktopPresentation::Canvas),
        (
            RemoteDesktopBackendPreference::WindowsNative,
            WindowsNativeRdpAvailability::UnavailableNotBuilt,
        ) => Err(RemoteDesktopPresentationError::UnavailableNotBuilt),
    }
}
