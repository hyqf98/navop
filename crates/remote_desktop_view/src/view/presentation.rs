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

/// A pre-connect reason that makes the native backend predictably unavailable.
///
/// Only this classification may trigger the `Auto` preference's Canvas
/// fallback. Runtime authentication, certificate, Gateway, server-policy, and
/// network failures never enter this model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowsNativeRdpUnavailableReason {
    FeatureDisabled,
    UnsupportedPlatform,
    ProbeReportedUnavailable,
}

/// A stable, non-fallback classification for failures while probing the native
/// boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowsNativeRdpProbeFailure {
    InvalidArgument,
    AbiMismatch,
    AllocationFailed,
    Internal,
    WrongThread,
    CallbackInFlight,
    InvalidState,
    NativeReturnedNullHandle,
    NativeDidNotClearHandle,
    InvalidNativeResponse,
    UnexpectedNativeResult(i32),
    NativeHresult { result: i32, hresult: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowsNativeRdpCapability {
    Available,
    Unavailable(WindowsNativeRdpUnavailableReason),
    ProbeFailed(WindowsNativeRdpProbeFailure),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RemoteDesktopPresentationSelection {
    pub(crate) presentation: RemoteDesktopPresentation,
    pub(crate) fallback_reason: Option<WindowsNativeRdpUnavailableReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RemoteDesktopPresentationError {
    NativeUnavailable(WindowsNativeRdpUnavailableReason),
    NativeProbeFailed(WindowsNativeRdpProbeFailure),
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

pub(crate) const fn select_remote_desktop_presentation(
    platform: RemoteDesktopPlatform,
    preference: RemoteDesktopBackendPreference,
    native_capability: WindowsNativeRdpCapability,
) -> Result<RemoteDesktopPresentationSelection, RemoteDesktopPresentationError> {
    if matches!(platform, RemoteDesktopPlatform::Other) {
        return Ok(canvas_selection(None));
    }

    classify_windows_presentation(preference, native_capability)
}

#[cfg(target_os = "windows")]
pub(crate) const fn current_remote_desktop_platform() -> RemoteDesktopPlatform {
    RemoteDesktopPlatform::Windows
}

#[cfg(not(target_os = "windows"))]
pub(crate) const fn current_remote_desktop_platform() -> RemoteDesktopPlatform {
    RemoteDesktopPlatform::Other
}

const fn classify_windows_presentation(
    preference: RemoteDesktopBackendPreference,
    native_capability: WindowsNativeRdpCapability,
) -> Result<RemoteDesktopPresentationSelection, RemoteDesktopPresentationError> {
    match (preference, native_capability) {
        (RemoteDesktopBackendPreference::Canvas, _) => Ok(canvas_selection(None)),
        (
            RemoteDesktopBackendPreference::Auto | RemoteDesktopBackendPreference::WindowsNative,
            WindowsNativeRdpCapability::Available,
        ) => Ok(RemoteDesktopPresentationSelection {
            presentation: RemoteDesktopPresentation::NativeWindows,
            fallback_reason: None,
        }),
        (RemoteDesktopBackendPreference::Auto, WindowsNativeRdpCapability::Unavailable(reason)) => {
            Ok(canvas_selection(Some(reason)))
        }
        (
            RemoteDesktopBackendPreference::WindowsNative,
            WindowsNativeRdpCapability::Unavailable(reason),
        ) => Err(RemoteDesktopPresentationError::NativeUnavailable(reason)),
        (
            RemoteDesktopBackendPreference::Auto | RemoteDesktopBackendPreference::WindowsNative,
            WindowsNativeRdpCapability::ProbeFailed(failure),
        ) => Err(RemoteDesktopPresentationError::NativeProbeFailed(failure)),
    }
}

const fn canvas_selection(
    fallback_reason: Option<WindowsNativeRdpUnavailableReason>,
) -> RemoteDesktopPresentationSelection {
    RemoteDesktopPresentationSelection {
        presentation: RemoteDesktopPresentation::Canvas,
        fallback_reason,
    }
}
