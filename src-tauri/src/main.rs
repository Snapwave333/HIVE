// Prevents additional console window on Windows
#![windows_subsystem = "windows"]

#[tokio::main]
async fn main() {
    hive_lib::run();
}
