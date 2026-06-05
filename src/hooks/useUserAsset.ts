import { useMemo } from 'react';
import { invoke } from '../adapters/tauri';

export interface UserAssetEntry {
  id: string;
  name: string;
  assetType: string; // "image" | "music" | "video" | "voice"
  filePath: string;
  tags: string[];
  createdAt: number;
  fileSize: number;
}

export function useUserAsset() {
  return useMemo(() => ({
    importUserAsset: (sourcePath: string, assetType: string, name: string, tags: string[]) =>
      invoke<UserAssetEntry>('import_user_asset', { sourcePath, assetType, name, tags }),
    listUserAssets: (assetType?: string) =>
      invoke<UserAssetEntry[]>('list_user_assets', { assetType: assetType || null }),
    deleteUserAsset: (assetId: string) =>
      invoke<void>('delete_user_asset', { assetId }),
    getUserAssetPath: (assetId: string) =>
      invoke<string>('get_user_asset_path', { assetId }),
  }), []);
}
