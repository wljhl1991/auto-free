import { useMemo } from 'react';
import { invoke, listen } from '../adapters/tauri';

export function useGeneration() {
  return useMemo(() => ({
    getGenerationStatus: (gameId: string) => invoke<any>('get_generation_status', { gameId }),
    getActiveGenerations: () => invoke<string[]>('get_active_generations'),
    regenerateAsset: (gameId: string, assetRefId: string) => invoke<void>('regenerate_asset', { gameId, assetRefId }),
    exportGame: (gameId: string, outputPath: string) => invoke<string>('export_game', { gameId, outputPath }),
    onAssetReady: (callback: (event: any) => void) => listen('asset-ready', callback),
    onAssetFailed: (callback: (event: any) => void) => listen('asset-failed', callback),
    onGenerationProgress: (callback: (event: any) => void) => listen('generation-progress', callback),
    onGenerationComplete: (callback: (event: any) => void) => listen('generation-complete', callback),
  }), []);
}
