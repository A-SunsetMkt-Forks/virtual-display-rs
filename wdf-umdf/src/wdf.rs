#![allow(non_snake_case)]

use std::ffi::c_void;

use wdf_umdf_sys::{
    NTSTATUS, PCUNICODE_STRING, PCWDF_OBJECT_CONTEXT_TYPE_INFO, PDRIVER_OBJECT, PWDFDEVICE_INIT,
    PWDF_DRIVER_CONFIG, PWDF_OBJECT_ATTRIBUTES, WDFDEVICE, WDFDRIVER, WDFOBJECT, WDF_NO_HANDLE,
    WDF_NO_OBJECT_ATTRIBUTES, WDF_OBJECT_ATTRIBUTES, _WDF_PNPPOWER_EVENT_CALLBACKS,
};

use crate::IntoHelper;

#[derive(Debug, thiserror::Error)]
pub enum WdfError {
    #[error("{0}")]
    WdfFunctionNotAvailable(&'static str),
    #[error("{0}")]
    CallFailed(NTSTATUS),
    // this is required for success status for ()
    #[error("This is not an error, ignore it")]
    Success,
}

impl From<()> for WdfError {
    fn from(_: ()) -> Self {
        WdfError::Success
    }
}

impl From<WdfError> for NTSTATUS {
    fn from(value: WdfError) -> Self {
        use WdfError::*;
        match value {
            WdfFunctionNotAvailable(_) => Self::STATUS_NOT_FOUND,
            CallFailed(status) => status,
            Success => 0.into(),
        }
    }
}

impl From<NTSTATUS> for WdfError {
    fn from(value: NTSTATUS) -> Self {
        WdfError::CallFailed(value)
    }
}

macro_rules! WdfCall {
    ($name:ident ( $($args:expr),* )) => {{
        let fn_handle = {
            ::paste::paste! {
                const FN_INDEX: usize = ::wdf_umdf_sys::WDFFUNCENUM::[<$name TableIndex>].0 as usize;

                // validate that wdf function can be used
                let is_available = ::wdf_umdf_sys::WdfIsFunctionAvailable!($name);

                if is_available {
                    // SAFETY: Only immutable accesses are done to this
                    let fn_table = unsafe { ::wdf_umdf_sys::WdfFunctions_02033 };

                    // SAFETY: Read-only, initialized by the time we use it, and checked to be in bounds
                    let fn_handle = unsafe {
                        fn_table
                        .add(FN_INDEX)
                        .cast::<::wdf_umdf_sys::[<PFN_ $name:upper>]>()
                    };

                    // SAFETY: Ensured that this is present by if condition from `WdfIsFunctionAvailable!`
                    let fn_handle = unsafe { fn_handle.read() };
                    // SAFETY: All available function handles are not null
                    let fn_handle = unsafe { fn_handle.unwrap_unchecked() };

                    Ok(fn_handle)
                } else {
                    Err($crate::WdfError::WdfFunctionNotAvailable(concat!(stringify!($name), " is not available")))
                }
            }
        };

        if let Ok(fn_handle) = fn_handle {
            // SAFETY: Pointer to globals is always immutable
            let globals = unsafe { ::wdf_umdf_sys::WdfDriverGlobals };

            // SAFETY: None. User is responsible for safety and must use their own unsafe block
            Ok(fn_handle(globals, $($args),*))
        } else {
            // SAFETY: We checked if it was Ok above, and it clearly isn't
            Err(unsafe {
                fn_handle.unwrap_err_unchecked()
            })
        }
    }};
}

/// Unlike the official WDF_DECLARE_CONTEXT_TYPE macro, you only need to declare this on the actual data struct want to use
/// Safety is maintained through a RwLock of the underlying data
///
/// This generates associated fns init/get/drop/get_type_info on your $context_type with the same visibility
///
/// Example:
/// ```rust
/// pub struct IndirectDeviceContext {
///     device: WDFDEVICE,
/// }
///
/// impl IndirectDeviceContext {
///     pub fn new(device: WDFDEVICE) -> Self {
///         Self { device }
///     }
/// }
///
/// WDF_DECLARE_CONTEXT_TYPE!(pub IndirectDeviceContext);
///
/// // with a `device: WDFDEVICE`
/// let context = IndirectDeviceContext::new(device as WDFOBJECT);
/// IndirectDeviceContext::init(context);
/// // elsewhere
/// let mutable_access = IndirectDeviceContext::get_mut(device).unwrap();
/// ```
#[macro_export]
macro_rules! WDF_DECLARE_CONTEXT_TYPE {
    ($sv:vis $context_type:ident) => {
        $crate::paste! {
            // keep it in a mod block to disallow access to private types
            mod [<WdfObject $context_type>] {
                use super::$context_type;

                // Require `T: Sync` for safety. User has to uphold the invariant themselves
                #[repr(transparent)]
                #[allow(non_camel_case_types)]
                struct [<_WDF_ $context_type _STATIC_WRAPPER>]<T> {
                    cell: ::std::cell::UnsafeCell<$crate::wdf_umdf_sys::_WDF_OBJECT_CONTEXT_TYPE_INFO>,
                    _phantom: ::std::marker::PhantomData<T>
                }

                // SAFETY: `T` impls Sync too
                unsafe impl<T: Sync> Sync for [<_WDF_ $context_type _STATIC_WRAPPER>]<T> {}

                // Unsure if C mutates this data, but it's in an unsafecell just in case
                #[allow(non_upper_case_globals)]
                static [<_WDF_ $context_type _TYPE_INFO>]: [<_WDF_ $context_type _STATIC_WRAPPER>]<$context_type> =
                    [<_WDF_ $context_type _STATIC_WRAPPER>] {
                        cell: ::std::cell::UnsafeCell::new(
                            $crate::wdf_umdf_sys::_WDF_OBJECT_CONTEXT_TYPE_INFO {
                                Size: ::std::mem::size_of::<$crate::wdf_umdf_sys::_WDF_OBJECT_CONTEXT_TYPE_INFO>() as u32,
                                ContextName: concat!(stringify!($context_type), "\0")
                                    .as_ptr().cast::<::std::ffi::c_char>(),
                                ContextSize: ::std::mem::size_of::<[<WdfObject $context_type>]>(),
                                // SAFETY:
                                // StaticWrapper and UnsafeCell are both repr(transparent), so cast to underlying _WDF_OBJECT_CONTEXT_TYPE_INFO is ok
                                UniqueType: &[<_WDF_ $context_type _TYPE_INFO>] as *const _ as *const _,
                                EvtDriverGetUniqueContextType: ::std::option::Option::None,
                            }
                        ),

                        _phantom: ::std::marker::PhantomData
                    };

                #[repr(transparent)]
                struct [<WdfObject $context_type>](::std::option::Option<::std::boxed::Box<::std::sync::RwLock<$context_type>>>);

                impl $context_type {
                    /// Initialize and place context into internal WdfObject
                    ///
                    /// SAFETY:
                    /// - handle must be a fresh unused object with no data in its context already
                    /// - context type must already have been set up for handle
                    $sv unsafe fn init(
                        self,
                        handle: $crate::wdf_umdf_sys::WDFOBJECT,
                    ) -> ::std::result::Result<(), $crate::WdfError> {
                        let context = unsafe {
                            $crate::WdfObjectGetTypedContextWorker(handle, [<_WDF_ $context_type _TYPE_INFO>].cell.get())?
                        } as *mut [<WdfObject $context_type>];

                        let mut_ref = &mut *context;
                        // initialize it to default data
                        mut_ref.0 = ::std::option::Option::Some(::std::boxed::Box::new(::std::sync::RwLock::new(self)));

                        Ok(())
                    }

                    /// SAFETY:
                    /// - Must not drop if it's still in use elsewhere
                    /// - No other mutable/non-mutable refs can exist to data when this is called, or it will alias
                    ///
                    /// This will overwrite data contained for this type
                    $sv unsafe fn drop(
                        handle: $crate::wdf_umdf_sys::WDFOBJECT,
                    ) -> ::std::result::Result<(), $crate::WdfError> {
                        let context = $crate::WdfObjectGetTypedContextWorker(
                            handle,
                            [<_WDF_ $context_type _TYPE_INFO>].cell.get(),
                        )? as *mut [<WdfObject $context_type>];

                        let mut_ref = &mut *context;
                        // Set to none so it can drop
                        mut_ref.0 = ::std::option::Option::None;

                        Ok(())
                    }

                    /// Get the context from the wdfobject.
                    /// Make sure you initialized it first, otherwise it will be unusable
                    ///
                    /// SAFETY:
                    /// - Must have initialized WdfObject first
                    /// - Data must not have been dropped
                    $sv unsafe fn get<'a>(
                        handle: *mut $crate::wdf_umdf_sys::WDFDEVICE__,
                    ) -> ::std::option::Option<::std::sync::RwLockReadGuard<'a, $context_type>>
                    {
                        let context = unsafe {
                            $crate::WdfObjectGetTypedContextWorker(handle as *mut _,
                                // SAFETY: Reading is always fine, since user cannot obtain mutable reference
                                (&*[<_WDF_ $context_type _TYPE_INFO>].cell.get()).UniqueType
                            ).ok()?
                        } as *mut [<WdfObject $context_type>];

                        unsafe { &*context }.0.as_ref().map(|r| r.read().ok()).flatten()
                    }

                    /// Get the context from the wdfobject.
                    /// Make sure you initialized it first, otherwise it will be unusable
                    ///
                    /// SAFETY:
                    /// - Must have initialized WdfObject first
                    /// - Data must not have been dropped
                    $sv unsafe fn get_mut<'a>(
                        handle: *mut $crate::wdf_umdf_sys::WDFDEVICE__,
                    ) -> ::std::option::Option<::std::sync::RwLockWriteGuard<'a, $context_type>>
                    {
                        let context = unsafe {
                            $crate::WdfObjectGetTypedContextWorker(handle as *mut _,
                                // SAFETY: Reading is always fine, since user cannot obtain mutable reference
                                (&*[<_WDF_ $context_type _TYPE_INFO>].cell.get()).UniqueType
                            ).ok()?
                        } as *mut [<WdfObject $context_type>];

                        unsafe { &*context }.0.as_ref().map(|r| r.write().ok()).flatten()
                    }

                    /// Get the context from the wdfobject.
                    /// Make sure you initialized it first, otherwise it will be unusable
                    ///
                    /// SAFETY:
                    /// - Must have initialized WdfObject first
                    /// - Data must not have been dropped
                    $sv unsafe fn try_get<'a>(
                        handle: *mut $crate::wdf_umdf_sys::WDFDEVICE__,
                    ) -> ::std::option::Option<::std::sync::RwLockReadGuard<'a, $context_type>>
                    {
                        let context = unsafe {
                            $crate::WdfObjectGetTypedContextWorker(handle as *mut _,
                                // SAFETY: Reading is always fine, since user cannot obtain mutable reference
                                (&*[<_WDF_ $context_type _TYPE_INFO>].cell.get()).UniqueType
                            ).ok()?
                        } as *mut [<WdfObject $context_type>];

                        unsafe { &*context }.0.as_ref().map(|r| r.try_read().ok()).flatten()
                    }

                    /// Get the context from the wdfobject.
                    /// Make sure you initialized it first, otherwise it will be unusable
                    ///
                    /// SAFETY:
                    /// - Must have initialized WdfObject first
                    /// - Data must not have been dropped
                    $sv unsafe fn try_get_mut<'a>(
                        handle: *mut $crate::wdf_umdf_sys::WDFDEVICE__,
                    ) -> ::std::option::Option<::std::sync::RwLockWriteGuard<'a, $context_type>>
                    {
                        let context = unsafe {
                            $crate::WdfObjectGetTypedContextWorker(handle as *mut _,
                                // SAFETY: Reading is always fine, since user cannot obtain mutable reference
                                (&*[<_WDF_ $context_type _TYPE_INFO>].cell.get()).UniqueType
                            ).ok()?
                        } as *mut [<WdfObject $context_type>];

                        unsafe { &*context }.0.as_ref().map(|r| r.try_write().ok()).flatten()
                    }

                    // SAFETY:
                    // - No other mutable refs must exist to target type
                    // - Underlying memory must remain immutable and unchanged until reference is dropped
                    $sv unsafe fn get_type_info() -> &'static $crate::wdf_umdf_sys::_WDF_OBJECT_CONTEXT_TYPE_INFO {
                        unsafe { &*[<_WDF_ $context_type _TYPE_INFO>].cell.get() }
                    }
                }
            }
        }
    };
}

pub unsafe fn WdfDriverCreate(
    // in
    DriverObject: PDRIVER_OBJECT,
    // in
    RegistryPath: PCUNICODE_STRING,
    // in, optional
    DriverAttributes: Option<PWDF_OBJECT_ATTRIBUTES>,
    // in
    DriverConfig: PWDF_DRIVER_CONFIG,
    // out, optional
    Driver: Option<&mut WDFDRIVER>,
) -> Result<NTSTATUS, WdfError> {
    WdfCall! {
        WdfDriverCreate(
            DriverObject,
            RegistryPath,
            DriverAttributes.unwrap_or(WDF_NO_OBJECT_ATTRIBUTES!()),
            DriverConfig,
            Driver
                .map(|d| d as *mut _)
                .unwrap_or(WDF_NO_HANDLE!() as *mut *mut _)
        )
    }
    .into_result()
}

pub unsafe fn WdfDeviceCreate(
    // in, out
    DeviceInit: &mut PWDFDEVICE_INIT,
    // in, optional
    DeviceAttributes: Option<&mut WDF_OBJECT_ATTRIBUTES>,
    // out
    Device: &mut WDFDEVICE,
) -> Result<NTSTATUS, WdfError> {
    WdfCall! {
        WdfDeviceCreate(
            DeviceInit,
            DeviceAttributes.map(|d| d as *mut _).unwrap_or(WDF_NO_OBJECT_ATTRIBUTES!() as *mut _),
            Device
        )
    }
    .into_result()
}

pub unsafe fn WdfDeviceInitSetPnpPowerEventCallbacks(
    // in
    DeviceInit: PWDFDEVICE_INIT,
    // in
    PnpPowerEventCallbacks: *mut _WDF_PNPPOWER_EVENT_CALLBACKS,
) -> Result<(), WdfError> {
    WdfCall! {
        WdfDeviceInitSetPnpPowerEventCallbacks(
            DeviceInit,
            PnpPowerEventCallbacks
        )
    }
}

pub unsafe fn WdfObjectGetTypedContextWorker(
    // in
    Handle: WDFOBJECT,
    // in
    TypeInfo: PCWDF_OBJECT_CONTEXT_TYPE_INFO,
) -> Result<*mut c_void, WdfError> {
    WdfCall! {
        WdfObjectGetTypedContextWorker(
            Handle,
            TypeInfo
        )
    }
}
