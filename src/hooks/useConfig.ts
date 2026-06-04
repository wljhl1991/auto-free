import { useMemo } from 'react';
import { invoke } from '../adapters/tauri';

export function useConfig() {
  return useMemo(() => ({
    getConfig: () => invoke<any>('get_config'),
    updateConfig: (config: any) => invoke<void>('update_config', { config }),
    getPresets: () => invoke<any[]>('get_presets'),
    applyPreset: (presetId: string) => invoke<void>('apply_preset', { presetId }),
    getProviders: () => invoke<any[]>('get_providers'),
    updateProvider: (provider: any) => invoke<void>('update_provider', { provider }),
    checkProvider: (providerId: string) => invoke<any>('check_provider', { providerId }),
    checkAllProviders: () => invoke<any[]>('check_all_providers'),
    exportConfig: () => invoke<string>('export_config'),
    importConfig: (configJson: string) => invoke<void>('import_config', { configJson }),
  }), []);
}
