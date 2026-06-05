import { useMemo } from 'react';
import { invoke, listen } from '../adapters/tauri';

export interface GenerationStepEvent {
  gameId: string;
  step: string;
  detail: string;
  modelName: string;
  timestamp: number;
}

export interface ChapterReadyEvent {
  gameId: string;
  chapterIndex: number;
  totalChapters: number;
  chapterId: string;
  chapterTitle: string;
}

export function useGeneration() {
  return useMemo(() => ({
    getGenerationStatus: (gameId: string) => invoke<any>('get_generation_status', { gameId }),
    getActiveGenerations: () => invoke<string[]>('get_active_generations'),
    regenerateAsset: (gameId: string, assetRefId: string) => invoke<void>('regenerate_asset', { gameId, assetRefId }),
    exportGame: (gameId: string, outputPath: string) => invoke<string>('export_game', { gameId, outputPath }),
    startRemainingChapters: (gameId: string) => invoke<void>('start_remaining_chapters', { gameId }),
    cancelRemainingChapters: (gameId: string) => invoke<void>('cancel_remaining_chapters', { gameId }),
    onAssetReady: (callback: (event: any) => void) => listen('asset-ready', callback),
    onAssetFailed: (callback: (event: any) => void) => listen('asset-failed', callback),
    onGenerationProgress: (callback: (event: any) => void) => listen('generation-progress', callback),
    onGenerationComplete: (callback: (event: any) => void) => listen('generation-complete', callback),
    onGenerationStep: (callback: (event: any) => void) => listen('generation-step', callback),
    onChapterReady: (callback: (event: any) => void) => listen('chapter-ready', callback),
    onAllChaptersReady: (callback: (event: any) => void) => listen('all-chapters-ready', callback),
  }), []);
}
