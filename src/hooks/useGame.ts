import { useMemo } from 'react';
import { invoke } from '../adapters/tauri';

export interface GameInfo {
  id: string;
  title: string;
  gameType: string;
  totalChapters: number;
  createdAt: number;
  updatedAt: number;
}

export function useGame() {
  return useMemo(() => ({
    createGame: (input: string, gameType?: string, useLocalFallback?: boolean) => invoke<GameInfo>('create_game', { input, gameType: gameType || null, useLocalFallback: useLocalFallback ?? null }),
    createGameFromScript: (scriptJson: string) => invoke<GameInfo>('create_game_from_script', { scriptJson }),
    getRandomOutline: (gameType?: string, themes?: string[]) => invoke<string>('random_outline', { gameType: gameType || null, themes: themes || [] }),
    getGame: (gameId: string) => invoke<GameInfo>('get_game', { gameId }),
    getGameScript: (gameId: string) => invoke<any>('get_game_script', { gameId }),
    listGames: () => invoke<GameInfo[]>('list_games'),
    deleteGame: (gameId: string) => invoke<void>('delete_game', { gameId }),
    saveGame: (gameId: string, state: any) => invoke<string>('save_game', { gameId, state }),
    loadSave: (gameId: string, saveId: string) => invoke<any>('load_save', { gameId, saveId }),
    listSaves: (gameId: string) => invoke<any[]>('list_saves', { gameId }),
  }), []);
}
