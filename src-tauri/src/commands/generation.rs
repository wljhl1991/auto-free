use tauri::command;
use crate::engine::pipeline::GenerationPipeline;
use crate::engine::asset_manager::AssetManager;
use crate::types::asset::LocalAsset;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::Path;
use std::io::Write;
use zip::write::SimpleFileOptions;

#[command]
pub async fn get_generation_status(
    game_id: String,
    pipeline: tauri::State<'_, Arc<RwLock<GenerationPipeline>>>,
) -> Result<crate::engine::pipeline::GenerationStatus, String> {
    let p = pipeline.read().await;
    p.get_status(&game_id).await
        .ok_or_else(|| format!("Generation status not found for game '{}'", game_id))
}

#[command]
pub async fn regenerate_asset(
    game_id: String,
    asset_ref_id: String,
    pipeline: tauri::State<'_, Arc<RwLock<GenerationPipeline>>>,
) -> Result<(), String> {
    let p = pipeline.read().await;
    p.regenerate_asset(&game_id, &asset_ref_id).await
        .map_err(|e| format!("{:?}", e))
}

#[command]
pub async fn regenerate_asset_candidates(
    game_id: String,
    asset_ref_id: String,
    count: Option<u32>,
    pipeline: tauri::State<'_, Arc<RwLock<GenerationPipeline>>>,
) -> Result<Vec<LocalAsset>, String> {
    let p = pipeline.read().await;
    let c = count.unwrap_or(3);
    p.regenerate_asset_with_candidates(&game_id, &asset_ref_id, c).await
        .map_err(|e| format!("{:?}", e))
}

#[command]
pub async fn export_game(
    game_id: String,
    output_path: String,
    asset_manager: tauri::State<'_, Arc<AssetManager>>,
) -> Result<String, String> {
    let game_dir = asset_manager.get_game_asset_dir(&game_id);
    if !game_dir.exists() {
        return Err(format!("Game directory not found: {:?}", game_dir));
    }

    let output = Path::new(&output_path);
    let file = std::fs::File::create(output)
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    add_dir_to_zip(&mut zip, &game_dir, &game_dir, &options)?;

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip archive: {}", e))?;

    Ok(output_path)
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base_dir: &Path,
    current_dir: &Path,
    options: &SimpleFileOptions,
) -> Result<(), String> {
    let entries = std::fs::read_dir(current_dir)
        .map_err(|e| format!("Failed to read directory {:?}: {}", current_dir, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        let relative = path.strip_prefix(base_dir)
            .map_err(|e| format!("Path strip error: {}", e))?;
        let relative_str = relative.to_string_lossy().to_string();

        if path.is_dir() {
            let dir_name = format!("{}/", relative_str);
            zip.add_directory(&dir_name, *options)
                .map_err(|e| format!("Failed to add directory '{}': {}", dir_name, e))?;
            add_dir_to_zip(zip, base_dir, &path, options)?;
        } else {
            zip.start_file(&relative_str, *options)
                .map_err(|e| format!("Failed to add file '{}': {}", relative_str, e))?;
            let data = std::fs::read(&path)
                .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
            zip.write_all(&data)
                .map_err(|e| format!("Failed to write file data: {}", e))?;
        }
    }

    Ok(())
}
