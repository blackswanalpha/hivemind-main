//! HiveMind desktop — Tauri bootstrap.
//!
//! Step 04 wires tab/session commands and emits `AppStarted` once the
//! SQLite-backed state is ready. AI commands land in step 07+.

mod ai_commands;
mod commands;
mod state;

use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let handle = app.handle().clone();
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("hivemind=debug,info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}
