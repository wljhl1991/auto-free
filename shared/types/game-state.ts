// GameState 类型定义 — 游戏运行时状态
// 对应 GAME_DESIGN.md §6.1

import type { GameScript } from './game-script';

export interface GenerationProgress {
  totalAssets: number;
  completedAssets: number;
  failedAssets: number;
  chapterStatus: Record<string, 'generating' | 'ready' | 'partial'>;
}

export interface ChoiceRecord {
  choiceNodeId: string;
  selectedOptionIndex: number;
  selectedOptionText: string;
  timestamp: number;
  chapterId: string;
  sceneId: string;
}

export interface GameState {
  saveId: string;
  gameScript: GameScript;
  currentChapterId: string;
  currentSceneId: string;
  currentNodeId: string;
  variables: Record<string, any>;
  inventory: string[];
  stats: Record<string, number>;
  choiceHistory: ChoiceRecord[];
  visitedScenes: string[];
  unlockedCGs: string[];
  generationProgress: GenerationProgress;
}
