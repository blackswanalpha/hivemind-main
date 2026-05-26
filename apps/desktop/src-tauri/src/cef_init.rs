//! CEF (Chromium Embedded Framework) initialization for HiveMind.
//!
//! Phase A goal: prove CEF can coexist with Tauri's tao event loop on Linux.
//! Browser hosts and tab plumbing land in Phase B (see `docs/plan-cef.md`).
//!
//! Threading model on Linux:
//! - The main (GTK / tao) thread owns both webkit2gtk and CEF UI work.
//! - `external_message_pump=true` means CEF will *ask* us to pump the loop
//!   via [`BrowserProcessHandler::on_schedule_message_pump_work`]. We must
//!   call [`cef::do_message_loop_work`] on the main thread after the
//!   requested delay — and *only* when asked. Pumping on every tao
//!   iteration interferes with webkit2gtk repaints and produces a blank
//!   Tauri window.

use std::sync::OnceLock;
use std::time::Duration;

use cef::*;
use tauri::AppHandle;

/// Set by `lib.rs::run` after Tauri has built. Read by
/// [`HiveMindBrowserProcessHandler::on_schedule_message_pump_work`] to
/// post pump work onto the main thread. None until Tauri is up; pump
/// requests that fire earlier are dropped (CEF retries).
pub static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Marker returned by [`load_cef`]; keep it alive for the lifetime of the
/// process. On macOS this owns the dynamic library handle; on Linux/Windows
/// it is a unit struct since libcef is linked at build time.
#[cfg(target_os = "macos")]
pub type CefLibrary = library_loader::LibraryLoader;

#[cfg(not(target_os = "macos"))]
pub struct CefLibrary;

/// Load libcef and pin the API version. Must be called once at process start,
/// before any other CEF call.
pub fn load_cef() -> CefLibrary {
    #[cfg(target_os = "macos")]
    let library = {
        let loader =
            library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), false);
        assert!(loader.load(), "failed to load libcef");
        loader
    };
    #[cfg(not(target_os = "macos"))]
    let library = CefLibrary;

    // Pin the API version so later cef calls use the matching ABI.
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    library
}

// Minimal CefApp for HiveMind. Phase A has no browser-process behaviour
// beyond logging context init + scheduling external pumps; Phase B wires
// per-tab `Client`s and creates browsers on demand.
wrap_app! {
    pub struct HiveMindCefApp;

    impl App {
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(HiveMindBrowserProcessHandler::new())
        }
    }
}

wrap_browser_process_handler! {
    struct HiveMindBrowserProcessHandler {}

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            tracing::info!("CEF context initialized");
        }

        fn on_schedule_message_pump_work(&self, delay_ms: i64) {
            // CEF wants `cef::do_message_loop_work()` called on the UI
            // thread in roughly `delay_ms` ms. We can't call it from this
            // callback (wrong thread, and CEF wants us to coalesce); post
            // through Tauri's main-thread dispatcher instead.
            let Some(handle) = APP_HANDLE.get().cloned() else {
                return;
            };
            let delay = Duration::from_millis(delay_ms.max(0) as u64);
            std::thread::spawn(move || {
                if !delay.is_zero() {
                    std::thread::sleep(delay);
                }
                let _ = handle.run_on_main_thread(|| {
                    cef::do_message_loop_work();
                });
            });
        }
    }
}

/// Settings for the main CEF process. `external_message_pump` is the
/// non-negotiable bit on Linux — it lets Tauri's tao loop drive CEF instead
/// of CEF blocking the thread with its own message loop.
pub fn settings() -> Settings {
    Settings {
        no_sandbox: 1,
        external_message_pump: 1,
        ..Default::default()
    }
}
