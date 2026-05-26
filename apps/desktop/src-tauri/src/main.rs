// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cef::*;
use hivemind_desktop_lib::cef_init;

fn main() -> Result<(), String> {
    // Must run before any other CEF call.
    let _library = cef_init::load_cef();

    // Parse CEF/Chromium command-line arguments (works for both browser
    // and subprocess invocations).
    let args = args::Args::new();
    let cmd_line = args
        .as_cmd_line()
        .ok_or_else(|| "failed to parse CEF command line".to_string())?;

    // CEF re-execs this same binary for renderer / gpu / utility helpers,
    // adding `--type=...`. Detect that role before doing anything Tauri.
    let type_switch = CefString::from("type");
    let is_browser_process = cmd_line.has_switch(Some(&type_switch)) != 1;

    // For the browser process this returns -1 immediately. For subprocesses
    // it runs the helper to completion and returns the exit code.
    let ret = execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());

    if !is_browser_process {
        if ret < 0 {
            return Err(format!(
                "CEF subprocess execute_process returned {ret}"
            ));
        }
        return Ok(());
    }

    if ret != -1 {
        return Err(format!(
            "CEF execute_process unexpectedly returned {ret} in browser process"
        ));
    }

    // Browser process: hand off to the lib, which initializes CEF and
    // builds the Tauri app.
    hivemind_desktop_lib::run(args);
    Ok(())
}
