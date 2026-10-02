//! PC Desktop Runner for Pomelo OS Applications.
//!
//! Initializes the simulated hardware Board, scans precompiled Wasm applications
//! from the local apps directory, determines the default boot application from
//! `apps.toml`, and launches the Pomelo OS Runtime.

use std::sync::Arc;
use pomelo_hal::Board;
use pomelo_runtime::{AppRegistry, program};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = std::env::args().skip(1);
    let target_app = args.next();

    // 1. Locate local apps directory (auto-bundle if missing)
    let candidates = [
        "dist/apps",
        "./dist/apps",
        "../dist/apps",
        "pomelo-apps/dist/apps",
        "../pomelo-apps/dist/apps",
        "/storage/apps",
    ];

    let mut apps_dir = None;
    for cand in &candidates {
        if std::path::Path::new(cand).exists() {
            apps_dir = Some(*cand);
            break;
        }
    }

    if apps_dir.is_none() {
        if std::path::Path::new("./bundle.sh").is_file() {
            println!("No dist/apps found. Automatically bundling applications...");
            let _ = std::process::Command::new("./bundle.sh").status();
            if std::path::Path::new("dist/apps").exists() {
                apps_dir = Some("dist/apps");
            }
        }
    }

    let apps_path = apps_dir.ok_or_else(|| {
        "Could not find 'dist/apps'. Please run './bundle.sh' inside 'pomelo-apps' first."
    })?;

    println!("==================================================");
    println!("  Pomelo OS - WebAssembly Desktop Simulator");
    println!("==================================================");
    println!("Apps directory: {apps_path}");

    // 2. Create HAL instance (simulated desktop board)
    let board = Arc::new(Board::simulated());

    // 3. Scan local apps path and create AppRegistry
    let registry = AppRegistry::from_dir(apps_path)?;
    println!("Discovered apps: {:?}", registry.list_apps());

    // 4. Resolve default application from apps.toml or CLI argument
    let default_app = target_app
        .or_else(|| registry.default_app().map(|s| s.to_string()))
        .unwrap_or_else(|| "app-launcher".to_string());

    println!("Booting into: '{default_app}' (480x480 window)");
    println!("Controls:");
    println!("  - Mouse: Click interactive UI elements");
    println!("  - Q key: Back to previous application");
    println!("  - W key: Return to Home (Launcher)");
    println!("==================================================");

    // 5. Start Pomelo OS Runtime
    program(board, registry, &default_app)
        .window(iced::window::Settings {
            size: iced::Size::new(480.0, 480.0),
            resizable: true,
            ..Default::default()
        })
        .run()?;

    Ok(())
}
