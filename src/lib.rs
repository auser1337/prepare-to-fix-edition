pub mod offsets;

use minhook::MinHook;
use parking_lot::Mutex;
use std::ffi::c_void;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use std::{mem, ptr};
use tracing::info;
use tracing_subscriber::fmt;
use windows::core::{Interface, BOOL};
use windows::Win32::Foundation::{HINSTANCE, HMODULE, HWND, TRUE};
use windows::Win32::Graphics::Direct3D9::{
    Direct3DCreate9, IDirect3D9, IDirect3DDevice9, D3DDEVTYPE,
    D3DPRESENT_INTERVAL_IMMEDIATE, D3DPRESENT_PARAMETERS, D3D_SDK_VERSION,
};
use windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows::Win32::System::Memory::{
    VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::System::Threading::{CreateThread, THREAD_CREATE_RUN_IMMEDIATELY};

type UpdateFn = extern "system" fn() -> bool;
type DisplayLogosFn = extern "fastcall" fn(*mut c_void, *const c_void, f32) -> bool;
type CreateDeviceFn = extern "stdcall" fn(
    *mut IDirect3D9,
    u32,
    D3DDEVTYPE,
    HWND,
    u32,
    *mut D3DPRESENT_PARAMETERS,
    *mut *mut IDirect3DDevice9,
) -> i32;
type ResetFn = extern "stdcall" fn(*mut IDirect3DDevice9, *mut D3DPRESENT_PARAMETERS) -> i32;

static ORIGINAL_UPDATE: OnceLock<UpdateFn> = OnceLock::new();
static ORIGINAL_DISPLAY_LOGOS: OnceLock<DisplayLogosFn> = OnceLock::new();
static ORIGINAL_CREATE_DEVICE: OnceLock<CreateDeviceFn> = OnceLock::new();
static ORIGINAL_RESET: OnceLock<ResetFn> = OnceLock::new();

static LAST_FRAME_TIME: Mutex<Option<Instant>> = Mutex::new(None);

unsafe fn protect_rwx(address: usize, size: usize) -> windows::core::Result<()> {
    let mut old_protect: PAGE_PROTECTION_FLAGS = Default::default();
    unsafe {
        VirtualProtect(
            address as *const _,
            size,
            PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        )
    }
}

extern "system" fn update_hook() -> bool {
    let now = Instant::now();

    {
        let mut last_frame_guard = LAST_FRAME_TIME.lock();
        if let Some(last_frame_time) = *last_frame_guard {
            let delta = now.duration_since(last_frame_time);

            unsafe {
                ptr::write(
                    offsets::FPS_TIME_STEP_DOUBLE as *mut f64,
                    delta.as_secs_f64(),
                );
                ptr::write(
                    offsets::FPS_TIME_STEP_FLOAT as *mut f32,
                    delta.as_secs_f32(),
                );
            }
        }
        *last_frame_guard = Some(now);
    }

    ORIGINAL_UPDATE.get().unwrap()()
}

// More accurate than the original `sleep` implementation. Is this even needed?
extern "cdecl" fn sleep_hook(micros: u32) {
    spin_sleep::sleep(Duration::from_micros(micros as u64));
}

extern "fastcall" fn display_logos_hook(
    this: *mut c_void,
    edx: *const c_void,
    _time_step: f32,
) -> bool {
    ORIGINAL_DISPLAY_LOGOS.get().unwrap()(this, edx, f32::MAX)
}

extern "stdcall" fn reset_hook(
    device: *mut IDirect3DDevice9,
    presentation_parameters: *mut D3DPRESENT_PARAMETERS,
) -> i32 {
    info!("`IDirect3DDevice9::Reset` called!");

    unsafe {
        (*presentation_parameters).PresentationInterval = D3DPRESENT_INTERVAL_IMMEDIATE as u32;
        (*presentation_parameters).FullScreen_RefreshRateInHz = 0;
    }

    ORIGINAL_RESET.get().unwrap()(device, presentation_parameters)
}

extern "stdcall" fn create_device_hook(
    d3d: *mut IDirect3D9,
    adapter: u32,
    device_type: D3DDEVTYPE,
    focus_window: HWND,
    behaviour_flags: u32,
    presentation_parameters: *mut D3DPRESENT_PARAMETERS,
    returned_device_interface: *mut *mut IDirect3DDevice9,
) -> i32 {
    info!("`IDirect3D9::CreateDevice` called!");

    unsafe {
        (*presentation_parameters).PresentationInterval = D3DPRESENT_INTERVAL_IMMEDIATE as u32;
        (*presentation_parameters).FullScreen_RefreshRateInHz = 0;
    }

    let result = ORIGINAL_CREATE_DEVICE.get().unwrap()(
        d3d,
        adapter,
        device_type,
        focus_window,
        behaviour_flags,
        presentation_parameters,
        returned_device_interface,
    );

    // D3D_OK
    if result == 0 {
        let vtable = unsafe { *(*returned_device_interface as *const *const *const c_void) };

        let original_reset =
            unsafe { MinHook::create_hook(*vtable.add(16) as _, reset_hook as _) }.unwrap();
        info!("Hooked `IDirect3DDevice9::Reset` @ {:p}", original_reset);

        ORIGINAL_RESET.get_or_init(|| unsafe { mem::transmute(original_reset) });

        unsafe { MinHook::enable_all_hooks() }.unwrap();
    }

    result
}

extern "system" fn main(_: *mut c_void) -> u32 {
    let file_appender = tracing_appender::rolling::daily("logs", "prepare-to-fix-edition.log");
    let subscriber = fmt()
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_max_level(tracing::Level::INFO)
        .with_writer(file_appender)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    unsafe {
        // FPS cap

        protect_rwx(offsets::FPS_LIMIT_2, size_of::<f64>()).unwrap();
        protect_rwx(offsets::FPS_LIMIT, size_of::<u8>()).unwrap();

        ptr::write(offsets::FPS_LIMIT_2 as *mut f64, 240.0);
        ptr::write(offsets::FPS_LIMIT as *mut u8, 240);

        // Constants

        protect_rwx(offsets::FPS_TIME_STEP_DOUBLE, size_of::<f64>()).unwrap();
        protect_rwx(offsets::FPS_TIME_STEP_FLOAT, size_of::<f32>()).unwrap();
        protect_rwx(offsets::FPS_11E7CD8, size_of::<f64>()).unwrap();
        protect_rwx(offsets::FPS_11E7CE0, size_of::<f64>()).unwrap();

        ptr::write(offsets::FPS_11E7CD8 as *mut f64, 120.0);
        ptr::write(offsets::FPS_11E7CE0 as *mut f64, -120.0);
    }

    {
        let original =
            unsafe { MinHook::create_hook(offsets::UPDATE as _, update_hook as _) }.unwrap();
        info!("Hooked `Update` @ {:#x}", offsets::UPDATE);

        ORIGINAL_UPDATE.get_or_init(|| unsafe { mem::transmute(original) });
    }

    {
        let _ = unsafe { MinHook::create_hook(offsets::SLEEP as _, sleep_hook as _) }.unwrap();
        info!("Hooked `sleep` @ {:#x}", offsets::SLEEP);
    }

    {
        let original =
            unsafe { MinHook::create_hook(offsets::DISPLAY_LOGOS as _, display_logos_hook as _) }
                .unwrap();
        info!(
            "Hooked `NS_FRPG::FrpgMenuDlgLogo::DisplayLogos` @ {:#x}",
            offsets::DISPLAY_LOGOS
        );

        ORIGINAL_DISPLAY_LOGOS.get_or_init(|| unsafe { mem::transmute(original) });
    }

    {
        let d3d = unsafe { Direct3DCreate9(D3D_SDK_VERSION) }.unwrap();
        let vtable = unsafe { *(d3d.as_raw() as *const *const *const c_void) };
        let create_device = unsafe { *vtable.add(16) };

        let original =
            unsafe { MinHook::create_hook(create_device as _, create_device_hook as _) }.unwrap();
        info!("Hooked `IDirect3D9::CreateDevice` @ {:p}", create_device);

        ORIGINAL_CREATE_DEVICE.get_or_init(|| unsafe { mem::transmute(original) });
    }

    unsafe { MinHook::enable_all_hooks() }.unwrap();
    info!("Enabled all hooks!");

    0
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
extern "system" fn DllMain(module_handle: HINSTANCE, call_reason: u32, _: *mut ()) -> BOOL {
    if call_reason == DLL_PROCESS_ATTACH {
        let _ = unsafe { DisableThreadLibraryCalls(HMODULE::from(module_handle)) };
        let _ = unsafe {
            CreateThread(
                None,
                0,
                Some(main),
                None,
                THREAD_CREATE_RUN_IMMEDIATELY,
                None,
            )
        };
    }

    TRUE
}
