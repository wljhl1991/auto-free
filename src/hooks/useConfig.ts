import { useMemo } from 'react';
import { invoke } from '../adapters/tauri';

export interface ModalityAvailability {
  text: boolean;
  image: boolean;
  video: boolean;
  music: boolean;
  voice: boolean;
}

export function useConfig() {
  return useMemo(() => ({
    getConfig: () => invoke<any>('get_config'),
    updateConfig: (config: any) => invoke<void>('update_config', { config }),
    getPresets: () => invoke<any[]>('get_presets'),
    applyPreset: (presetId: string) => invoke<void>('apply_preset', { presetId }),
    getProviders: () => invoke<any[]>('get_providers'),
    updateProvider: (provider: any) => invoke<void>('update_provider', { provider }),
    checkProvider: (providerId: string, testPrompt?: string, modelId?: string, providerOverride?: any) => invoke<any>('check_provider', { providerId, testPrompt: testPrompt || null, modelId: modelId || null, providerOverride: providerOverride || null }),
    checkAllProviders: () => invoke<any[]>('check_all_providers'),
    checkAvailableModalities: () => invoke<ModalityAvailability>('check_available_modalities'),
    exportConfig: () => invoke<string>('export_config'),
    importConfig: (configJson: string) => invoke<void>('import_config', { configJson }),
    saveDevConfig: () => invoke<void>('save_dev_config'),
    loadDevConfig: () => invoke<void>('load_dev_config'),
    updateProviderModels: (providersJson: string) => invoke<void>('update_provider_models', { providersJson }),
    readLogs: (lines?: number) => invoke<string>('read_recent_logs', { lines: lines || null }),
    readCallHistory: (lines?: number) => invoke<string>('read_call_history', { lines: lines || null }),
    resetConfig: () => invoke<void>('reset_config'),
    getBuiltinProviderTemplates: () => invoke<any[]>('get_builtin_provider_templates'),
    deleteProvider: (providerId: string) => invoke<void>('delete_provider', { providerId }),
    copyProvider: (providerId: string) => invoke<string>('copy_provider', { providerId }),
    resetProvider: (providerId: string) => invoke<void>('reset_provider', { providerId }),
  }), []);
}
