mod types;
mod engine;
mod config;
mod providers;
mod commands;

use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tauri::Manager;

use commands::{game, generation, asset, logs, user_asset};
use commands::config as cmd_config;
use config::manager::ConfigManager;
use engine::asset_manager::AssetManager;
use engine::pipeline::GenerationPipeline;

fn get_gen_base_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("gen")
}

fn ensure_gen_dirs(base_path: &PathBuf) {
    let subdirs = ["logs", "games", "config", "cache", "exports", "user-assets", "ai-responses", "call-history"];
    for dir in &subdirs {
        let _ = std::fs::create_dir_all(base_path.join(dir));
    }
}

fn init_logging() {
    let log_dir = get_gen_base_path().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("autofree.log");

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .expect("无法打开日志文件");

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S%.3f]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stderr())
        .chain(file)
        .apply()
        .expect("初始化日志失败");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    let base_path = get_gen_base_path();
    ensure_gen_dirs(&base_path);

    let config_dir = base_path.join("config");

    let mut config_manager = ConfigManager::new(config_dir);
    if let Err(e) = config_manager.load() {
        eprintln!("Warning: Failed to load config: {}", e);
    }
    // 开发模式下从 dev-config.json 加载配置
    if let Err(e) = config_manager.load_dev_config() {
        eprintln!("Warning: Failed to load dev config: {}", e);
    }

    let asset_manager = AssetManager::new(base_path.clone());

    // 确保 user-assets 目录存在
    if let Err(e) = crate::commands::user_asset::ensure_user_assets_dirs(&base_path) {
        eprintln!("Warning: Failed to create user-assets dirs: {}", e);
    }

    let config_manager_arc = Arc::new(RwLock::new(config_manager));
    let asset_manager_arc = Arc::new(asset_manager);

    let pipeline = GenerationPipeline::new(config_manager_arc.clone(), asset_manager_arc.clone());
    let pipeline_arc = Arc::new(RwLock::new(pipeline));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(config_manager_arc)
        .manage(asset_manager_arc)
        .manage(pipeline_arc)
        .setup(|app| {
            let pipeline = app.state::<Arc<RwLock<GenerationPipeline>>>();
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async {
                pipeline.write().await.set_app_handle(handle);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            game::create_game,
            game::create_game_from_script,
            game::random_outline,
            game::get_game,
            game::get_game_script,
            game::list_games,
            game::delete_game,
            game::repair_game,
            game::save_game,
            game::load_save,
            game::list_saves,
            game::start_remaining_chapters,
            game::cancel_remaining_chapters,
            game::save_creation_history,
            game::get_creation_history,
            game::delete_creation_history,
            game::clear_creation_history,
            generation::get_generation_status,
            generation::get_active_generations,
            generation::regenerate_asset,
            generation::regenerate_asset_candidates,
            generation::export_game,
            cmd_config::get_config,
            cmd_config::update_config,
            cmd_config::get_presets,
            cmd_config::apply_preset,
            cmd_config::get_providers,
            cmd_config::update_provider,
            cmd_config::check_provider,
            cmd_config::check_all_providers,
            cmd_config::export_config,
            cmd_config::import_config,
            cmd_config::save_dev_config,
            cmd_config::load_dev_config,
            cmd_config::update_provider_models,
            cmd_config::check_available_modalities,
            cmd_config::reset_config,
            cmd_config::get_builtin_provider_templates,
            cmd_config::delete_provider,
            cmd_config::copy_provider,
            cmd_config::reset_provider,
            asset::get_asset_path,
            asset::list_builtin_assets,
            asset::read_file_as_data_url,
            user_asset::import_user_asset,
            user_asset::import_user_asset_from_data,
            user_asset::list_user_assets,
            user_asset::delete_user_asset,
            user_asset::get_user_asset_path,
            logs::get_log_path,
            logs::read_recent_logs,
            logs::read_call_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
