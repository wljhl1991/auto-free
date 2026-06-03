import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from '@/hooks/useConfig';
import type { AIProviderConfig, ConfigPreset, AIModality } from '@/types';
import PresetSelector from '@/components/Config/PresetSelector';
import ModalitySection from '@/components/Config/ModalitySection';
import ProviderConfigModal from '@/components/Config/ProviderConfigModal';

const MODALITY_SECTIONS: { modality: AIModality; title: string }[] = [
  { modality: 'text', title: '文本生成' },
  { modality: 'image', title: '图片生成' },
  { modality: 'video', title: '视频生成' },
  { modality: 'music', title: '音乐生成' },
  { modality: 'voice', title: '语音生成' },
];

export default function Settings() {
  const navigate = useNavigate();
  const config = useConfig();

  const [presets, setPresets] = useState<ConfigPreset[]>([]);
  const [activePresetId, setActivePresetId] = useState('');
  const [providers, setProviders] = useState<AIProviderConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);

  // Modal state
  const [modalProvider, setModalProvider] = useState<AIProviderConfig | null>(null);
  const [modalOpen, setModalOpen] = useState(false);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const [configData, presetsData, providersData] = await Promise.all([
        config.getConfig(),
        config.getPresets(),
        config.getProviders(),
      ]);
      setActivePresetId(configData?.activePresetId || '');
      setPresets(presetsData || []);
      setProviders(providersData || []);
    } catch (err) {
      console.error('Failed to load config:', err);
    } finally {
      setLoading(false);
    }
  }, [config]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleSelectPreset = async (presetId: string) => {
    try {
      await config.applyPreset(presetId);
      setActivePresetId(presetId);
      // Reload providers after applying preset
      const providersData = await config.getProviders();
      setProviders(providersData || []);
    } catch (err) {
      console.error('Failed to apply preset:', err);
    }
  };

  const handleConfigure = (providerId: string) => {
    const provider = providers.find((p) => p.id === providerId);
    if (provider) {
      setModalProvider({ ...provider });
      setModalOpen(true);
    }
  };

  const handleSaveProvider = async (updatedProvider: AIProviderConfig) => {
    try {
      await config.updateProvider(updatedProvider);
      setProviders((prev) =>
        prev.map((p) => (p.id === updatedProvider.id ? updatedProvider : p))
      );
    } catch (err) {
      console.error('Failed to save provider:', err);
    }
  };

  const handleCheckProvider = async (providerId: string) => {
    try {
      const result = await config.checkProvider(providerId);
      setProviders((prev) =>
        prev.map((p) => (p.id === providerId ? { ...p, ...result } : p))
      );
      if (modalProvider?.id === providerId) {
        setModalProvider((prev) => (prev ? { ...prev, ...result } : prev));
      }
    } catch (err) {
      console.error('Failed to check provider:', err);
    }
  };

  const handleCheckAll = async () => {
    try {
      setChecking(true);
      const results = await config.checkAllProviders();
      if (Array.isArray(results)) {
        setProviders((prev) =>
          prev.map((p) => {
            const result = results.find((r: any) => r.id === p.id);
            return result ? { ...p, ...result } : p;
          })
        );
      }
    } catch (err) {
      console.error('Failed to check all providers:', err);
    } finally {
      setChecking(false);
    }
  };

  const handleExport = async () => {
    try {
      const json = await config.exportConfig();
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'autofree-config.json';
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to export config:', err);
    }
  };

  const handleImport = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        await config.importConfig(text);
        await loadData();
      } catch (err) {
        console.error('Failed to import config:', err);
      }
    };
    input.click();
  };

  const handleReset = async () => {
    try {
      await config.applyPreset('default');
      await loadData();
    } catch (err) {
      console.error('Failed to reset config:', err);
    }
  };

  const getProvidersByModality = (modality: AIModality): AIProviderConfig[] => {
    return providers.filter((p) => p.modality.includes(modality));
  };

  if (loading) {
    return (
      <div className="page settings">
        <button className="btn btn-back" onClick={() => navigate('/')}>
          ← 返回
        </button>
        <h2 className="page-title">AI 配置管理</h2>
        <p className="placeholder-text">加载中...</p>
      </div>
    );
  }

  return (
    <div className="page settings">
      {/* Header */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '2rem',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <button className="btn btn-back" onClick={() => navigate('/')}>
            ← 返回
          </button>
          <h2 className="page-title" style={{ marginBottom: 0 }}>
            AI 配置管理
          </h2>
        </div>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <button
            className="btn btn-secondary"
            style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}
            onClick={handleExport}
          >
            导出配置
          </button>
          <button
            className="btn btn-secondary"
            style={{ padding: '0.5rem 1rem', fontSize: '0.85rem' }}
            onClick={handleImport}
          >
            导入配置
          </button>
        </div>
      </div>

      {/* Preset Selector */}
      <PresetSelector
        presets={presets}
        activePresetId={activePresetId}
        onSelect={handleSelectPreset}
      />

      {/* Modality Sections */}
      {MODALITY_SECTIONS.map(({ modality, title }) => (
        <ModalitySection
          key={modality}
          modality={modality}
          title={title}
          providers={getProvidersByModality(modality)}
          onConfigure={handleConfigure}
          onCheck={handleCheckProvider}
        />
      ))}

      {/* Bottom Actions */}
      <div style={{
        display: 'flex',
        gap: '0.75rem',
        justifyContent: 'center',
        marginTop: '2rem',
        paddingTop: '1.5rem',
        borderTop: '1px solid #2a2a3a',
      }}>
        <button
          className="btn btn-primary"
          onClick={handleCheckAll}
          disabled={checking}
          style={{ padding: '0.75rem 2rem' }}
        >
          {checking ? '检测中...' : '全部检测'}
        </button>
        <button
          className="btn btn-secondary"
          onClick={handleReset}
          style={{ padding: '0.75rem 2rem' }}
        >
          恢复默认
        </button>
      </div>

      {/* Provider Config Modal */}
      {modalProvider && (
        <ProviderConfigModal
          provider={modalProvider}
          isOpen={modalOpen}
          onClose={() => {
            setModalOpen(false);
            setModalProvider(null);
          }}
          onSave={handleSaveProvider}
          onCheck={() => handleCheckProvider(modalProvider.id)}
        />
      )}
    </div>
  );
}
