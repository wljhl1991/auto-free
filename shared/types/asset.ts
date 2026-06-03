// Asset 类型定义 — 本地资源管理
// 对应 GAME_DESIGN.md §4.1

export type AIModality = 'text' | 'image' | 'video' | 'music' | 'voice';

export type AssetType = 'image' | 'video' | 'audio' | 'voice';

export type AssetSource = 'ai_generated' | 'builtin' | 'local_file';

export interface LocalAsset {
  id: string;
  type: AssetType;
  localPath: string;
  source: AssetSource;
  cacheKey: string;
  createdAt: number;
}
