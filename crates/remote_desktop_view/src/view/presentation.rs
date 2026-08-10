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
    ClassNotRegistered,
    RequiredInterfaceMissing,
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RemoteDesktopPresentationCreation<T> {
    Canvas {
        fallback_reason: Option<WindowsNativeRdpUnavailableReason>,
    },
    Native(T),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RemoteDesktopPresentationCreateError<E> {
    Selection(RemoteDesktopPresentationError),
    NativeCreate(E),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RemoteDesktopPresentationInitialization {
    Pending,
    Canvas {
        fallback_reason: Option<WindowsNativeRdpUnavailableReason>,
    },
    Native,
    Failed,
}

impl RemoteDesktopPresentationInitialization {
    pub(crate) const fn allows_canvas_runtime(self) -> bool {
        matches!(self, Self::Canvas { .. })
    }
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

#[cfg(feature = "windows-native-rdp")]
pub(crate) const fn classify_windows_native_create_error(
    error: windows_rdp_host::WindowsRdpHostError,
) -> Option<WindowsNativeRdpUnavailableReason> {
    use windows_rdp_host::WindowsRdpHostError;

    const REGDB_E_CLASSNOTREG: i32 = 0x8004_0154_u32 as i32;
    const E_NOINTERFACE: i32 = 0x8000_4002_u32 as i32;

    match error {
        WindowsRdpHostError::NativeHresult { hresult, .. } => match hresult.code() {
            REGDB_E_CLASSNOTREG => Some(WindowsNativeRdpUnavailableReason::ClassNotRegistered),
            E_NOINTERFACE => Some(WindowsNativeRdpUnavailableReason::RequiredInterfaceMissing),
            _ => None,
        },
        _ => None,
    }
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

pub(crate) fn create_remote_desktop_presentation_with<T, E>(
    platform: RemoteDesktopPlatform,
    preference: RemoteDesktopBackendPreference,
    probe_native: impl FnOnce() -> WindowsNativeRdpCapability,
    create_native: impl FnOnce() -> Result<T, E>,
    classify_create_error: impl FnOnce(&E) -> Option<WindowsNativeRdpUnavailableReason>,
) -> Result<RemoteDesktopPresentationCreation<T>, RemoteDesktopPresentationCreateError<E>> {
    if matches!(platform, RemoteDesktopPlatform::Other)
        || matches!(preference, RemoteDesktopBackendPreference::Canvas)
    {
        return Ok(RemoteDesktopPresentationCreation::Canvas {
            fallback_reason: None,
        });
    }

    let selection = select_remote_desktop_presentation(platform, preference, probe_native())
        .map_err(RemoteDesktopPresentationCreateError::Selection)?;
    if matches!(selection.presentation, RemoteDesktopPresentation::Canvas) {
        return Ok(RemoteDesktopPresentationCreation::Canvas {
            fallback_reason: selection.fallback_reason,
        });
    }

    match create_native() {
        Ok(native) => Ok(RemoteDesktopPresentationCreation::Native(native)),
        Err(error) => {
            if matches!(preference, RemoteDesktopBackendPreference::Auto)
                && let Some(reason) = classify_create_error(&error)
            {
                return Ok(RemoteDesktopPresentationCreation::Canvas {
                    fallback_reason: Some(reason),
                });
            }
            Err(RemoteDesktopPresentationCreateError::NativeCreate(error))
        }
    }
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
