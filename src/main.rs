#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::process::exit(empty_folder_cleaner::shell::webview_host::run());
}
