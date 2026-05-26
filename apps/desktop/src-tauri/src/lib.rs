//! HiveMind desktop — Tauri bootstrap.
//!
//! Step 04 wires tab/session commands and emits `AppStarted` once the
//! SQLite-backed state is ready. AI commands land in step 07+.
//! Engine migration (Phase A): CEF is initialized before Tauri builds and
//! pumped from Tauri's run-event callback. See `cef_init.rs`.

mod ai_commands;
pub mod cef_init;
mod commands;
mod state;

use cef::args::Args;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(cef_args: Args) {
    init_tracing();

    // Initialize CEF before building the Tauri app. external_message_pump
    // mode lets Tauri's tao loop drive CEF; without it CEF would try to own
    // the main thread and deadlock the GTK loop.
    let mut cef_app = cef_init::HiveMindCefApp::new();
    let cef_settings = cef_init::settings();
    let init_ret = cef::initialize(
        Some(cef_args.as_main_args()),
        Some(&cef_settings),
        Some(&mut cef_app),
        std::ptr::null_mut(),
    );
    if init_ret != 1 {
        panic!("cef::initialize returned {init_ret} (expected 1)");
    }
    tracing::info!("CEF initialized");

    let tauri_app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let handle = app.handle().clone();
            // Publish the AppHandle so the CEF BrowserProcessHandler can
            // post pump work onto the main thread. Must happen as early as
            // possible in setup; until set, `on_schedule_message_pump_work`
            // drops requests (CEF retries).
            let _ = cef_init::APP_HANDLE.set(handle.clone());

            // Block until persistence is open and session is hydrated. This
            // happens before the first command can fire, so state::AppState
            // is guaranteed to be present.
            tauri::async_runtime::block_on(async move {
                if let Err(err) = state::initialize(&handle).await {
                    tracing::error!(error = ?err, "failed to initialize app state");
                    return Err(Box::<dyn std::error::Error>::from(err.to_string()));
                }
                Ok(())
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::load_session,
            commands::list_tabs,
            commands::open_tab,
            commands::close_tab,
            commands::set_active_tab,
            commands::navigate,
            commands::switch_workspace,
            ai_commands::create_conversation,
            ai_commands::list_conversations,
            ai_commands::load_messages,
            ai_commands::delete_conversation,
            ai_commands::send_message,
            ai_commands::list_providers,
            ai_commands::get_ai_settings,
            ai_commands::set_ai_settings,
            ai_commands::test_provider,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    tauri_app.run(|_app_handle, event| {
        // CEF is pumped on demand via
        // `HiveMindBrowserProcessHandler::on_schedule_message_pump_work`,
        // not on every Tauri event — the latter starves webkit2gtk repaints
        // and leaves the Tauri window blank.
        if matches!(event, tauri::RunEvent::Exit) {
            tracing::info!("Tauri Exit — shutting down CEF");
            cef::shutdown();
        }
    });
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("hivemind=debug,info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}
