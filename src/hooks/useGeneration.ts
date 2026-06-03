import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function useGeneration() {
  return {
    getGenerationStatus: (gameId: string) => invoke<any>('get_generation_status', { gameId }),
    regenerateAsset: (gameId: string, assetRefId: string) => invoke<void>('regenerate_asset', { gameId, assetRefId }),
    exportGame: (gameId: string, outputPath: string) => invoke<void>('export_game', { gameId, outputPath }),
    onAssetReady: (callback: (event: any) => void) => listen('asset-ready', callback),
    onAssetFailed: (callback: (event: any) => void) => listen('asset-failed', callback),
    onGenerationProgress: (callback: (event: any) => void) => listen('generation-progress', callback),
    onGenerationComplete: (callback: (event: any) => void) => listen('generation-complete', callback),
  };
}
