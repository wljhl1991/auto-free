import { useState } from 'react';
import type { AIProviderConfig, ProviderStatus } from '@/types';

interface ProviderConfigModalProps {
  provider: AIProviderConfig;
  isOpen: boolean;
  onClose: () => void;
  onSave: (provider: AIProviderConfig) => Promise<void>;
  onCheck: (providerId: string) => Promise<any>;
}

export default function ProviderConfigModal({
  provider,
  isOpen,
  onClose,
  onSave,
  onCheck,
}: ProviderConfigModalProps) {
  const [editedProvider, setEditedProvider] = useState<AIProviderConfig>(provider);
  const [showApiKey, setShowApiKey] = useState(false);
  const [checking, setChecking] = useState(false);
  const [checkResult, setCheckResult] = useState<{ status: string; message?: string } | null>(null);
  const [saving, setSaving] = useState(false);

  if (!isOpen) return null;

  const defaultModel = editedProvider.models.find(m => m.isDefault) || editedProvider.models[0];
  const currentEndpoint = defaultModel?.endpoint || '';

  const handleApiKeyChange = (value: string) => {
    setEditedProvider(prev => ({
      ...prev,
      authConfig: {
        ...prev.authConfig,
        apiKey: prev.authConfig.apiKey
          ? { ...prev.authConfig.apiKey, value }
          : { value, label: 'API Key', placeholder: '输入 API Key', helpUrl: '' },
      },
      status: value.trim() ? 'configured' : 'unconfigured',
    }));
    setCheckResult(null);
  };

  const handleEndpointChange = (value: string) => {
    setEditedProvider(prev => ({
      ...prev,
      models: prev.models.map(m => ({ ...m, endpoint: value })),
    }));
    setCheckResult(null);
  };

  const handleModelChange = (modelId: string) => {
    setEditedProvider(prev => ({
      ...prev,
      models: prev.models.map(m => ({
        ...m,
        isDefault: m.id === modelId,
      })),
    }));
    setCheckResult(null);
  };

  const handleExtraParamChange = (key: string, value: string) => {
    setEditedProvider(prev => ({
      ...prev,
      authConfig: {
        ...prev.authConfig,
        extraParams: {
          ...(prev.authConfig.extraParams || {}),
          [key]: {
            ...(prev.authConfig.extraParams?.[key] || { label: key, placeholder: '', required: false, secret: false }),
            value,
          },
        },
      },
      status: 'configured',
    }));
    setCheckResult(null);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave(editedProvider);
    } finally {
      setSaving(false);
    }
  };

  const handleCheck = async () => {
    // 先保存再测试
    setSaving(true);
    try {
      await onSave(editedProvider);
    } finally {
      setSaving(false);
    }

    setChecking(true);
    setCheckResult(null);
    try {
      const result = await onCheck(editedProvider.id);
      setCheckResult({
        status: result?.status || 'ok',
        message: result?.errorMessage,
      });
      if (result) {
        setEditedProvider(prev => ({
          ...prev,
          status: result.status === 'ok' ? 'connected' : mapCheckStatus(result.status),
          lastChecked: result.timestamp,
          errorMessage: result.errorMessage,
        }));
      }
    } catch (err: any) {
      const msg = typeof err === 'string' ? err : (err?.message || err?.toString() || '检测失败');
      setCheckResult({
        status: 'error',
        message: msg,
      });
    } finally {
      setChecking(false);
    }
  };

  const mapCheckStatus = (status: string): ProviderStatus => {
    const map: Record<string, ProviderStatus> = {
      'ok': 'connected',
      'auth_failed': 'auth_failed',
      'network_error': 'network_error',
      'quota_exceeded': 'quota_exceeded',
      'unconfigured': 'unconfigured',
    };
    return map[status] || 'error';
  };

  const getStatusLabel = (status: ProviderStatus): string => {
    const map: Record<ProviderStatus, string> = {
      connected: '已连接',
      configured: '已配置',
      unconfigured: '未配置',
      auth_failed: '认证失败',
      quota_exceeded: '额度不足',
      network_error: '网络错误',
      error: '错误',
    };
    return map[status] || '未知';
  };

  const getStatusColor = (status: ProviderStatus): string => {
    const map: Record<ProviderStatus, string> = {
      connected: '#4caf50',
      configured: '#ff9800',
      unconfigured: '#666680',
      auth_failed: '#e06060',
      quota_exceeded: '#ff9800',
      network_error: '#e06060',
      error: '#e06060',
    };
    return map[status] || '#666680';
  };

  return (
    <div style={{
      position: 'fixed', inset: 0,
      backgroundColor: 'rgba(0,0,0,0.7)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      zIndex: 1000,
    }} onClick={e => { if (e.target === e.currentTarget) onClose(); }}>
      <div style={{
        backgroundColor: '#16162a', border: '1px solid #2a2a3a',
        borderRadius: '12px', padding: '2rem',
        width: '90%', maxWidth: '560px', maxHeight: '85vh', overflowY: 'auto',
      }}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1.5rem' }}>
          <div>
            <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: '#e0e0f0', marginBottom: '0.25rem' }}>
              {editedProvider.name}
            </h3>
            <p style={{ fontSize: '0.85rem', color: '#8888aa' }}>{editedProvider.description}</p>
          </div>
          <span style={{
            display: 'inline-flex', alignItems: 'center', gap: '6px',
            fontSize: '0.8rem', color: '#aaaacc',
          }}>
            <span style={{
              width: '8px', height: '8px', borderRadius: '50%',
              backgroundColor: getStatusColor(editedProvider.status),
            }} />
            {getStatusLabel(editedProvider.status)}
          </span>
        </div>

        {/* API Key */}
        {editedProvider.authType === 'api_key' && editedProvider.authConfig.apiKey && (
          <div style={{ marginBottom: '1.25rem' }}>
            <label style={{ display: 'block', fontSize: '0.9rem', color: '#9999bb', marginBottom: '0.4rem' }}>
              {editedProvider.authConfig.apiKey.label || 'API Key'}
            </label>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <input
                type={showApiKey ? 'text' : 'password'}
                value={editedProvider.authConfig.apiKey.value || ''}
                onChange={e => handleApiKeyChange(e.target.value)}
                placeholder={editedProvider.authConfig.apiKey.placeholder || '输入 API Key'}
                style={{
                  flex: 1, padding: '0.6rem 0.8rem', fontSize: '0.9rem',
                  fontFamily: 'monospace', backgroundColor: '#0a0a1a', color: '#e0e0e0',
                  border: '1px solid #2a2a3a', borderRadius: '8px', outline: 'none',
                }}
              />
              <button className="btn btn-secondary" style={{ padding: '0.6rem 0.8rem', fontSize: '0.8rem', whiteSpace: 'nowrap' }}
                onClick={() => setShowApiKey(!showApiKey)}>
                {showApiKey ? '隐藏' : '显示'}
              </button>
            </div>
            {editedProvider.authConfig.apiKey.helpUrl && (
              <a href={editedProvider.authConfig.apiKey.helpUrl} target="_blank" rel="noopener noreferrer"
                style={{ display: 'inline-block', marginTop: '0.4rem', fontSize: '0.8rem', color: '#4a90d9', textDecoration: 'none' }}>
                获取 API Key →
              </a>
            )}
          </div>
        )}

        {/* Extra Params (e.g. 可灵的 Access Key / Secret Key, 讯飞的 AppID 等) */}
        {editedProvider.authConfig.extraParams && Object.entries(editedProvider.authConfig.extraParams).map(([key, field]) => (
          <div key={key} style={{ marginBottom: '1.25rem' }}>
            <label style={{ display: 'block', fontSize: '0.9rem', color: '#9999bb', marginBottom: '0.4rem' }}>
              {field.label}
            </label>
            <input
              type={field.secret ? 'password' : 'text'}
              value={field.value || ''}
              onChange={e => handleExtraParamChange(key, e.target.value)}
              placeholder={field.placeholder}
              style={{
                width: '100%', padding: '0.6rem 0.8rem', fontSize: '0.9rem',
                fontFamily: 'monospace', backgroundColor: '#0a0a1a', color: '#e0e0e0',
                border: '1px solid #2a2a3a', borderRadius: '8px', outline: 'none',
                boxSizing: 'border-box',
              }}
            />
          </div>
        ))}

        {/* API Endpoint */}
        <div style={{ marginBottom: '1.25rem' }}>
          <label style={{ display: 'block', fontSize: '0.9rem', color: '#9999bb', marginBottom: '0.4rem' }}>
            API 地址
          </label>
          <input
            type="text"
            value={currentEndpoint}
            onChange={e => handleEndpointChange(e.target.value)}
            placeholder="https://api.example.com/v1/chat/completions"
            style={{
              width: '100%', padding: '0.6rem 0.8rem', fontSize: '0.85rem',
              fontFamily: 'monospace', backgroundColor: '#0a0a1a', color: '#e0e0e0',
              border: '1px solid #2a2a3a', borderRadius: '8px', outline: 'none',
              boxSizing: 'border-box',
            }}
          />
          <span style={{ fontSize: '0.75rem', color: '#666680', marginTop: '0.25rem', display: 'block' }}>
            默认已填入，通常无需修改
          </span>
        </div>

        {/* Model Selection */}
        {editedProvider.models.length > 0 && (
          <div style={{ marginBottom: '1.25rem' }}>
            <label style={{ display: 'block', fontSize: '0.9rem', color: '#9999bb', marginBottom: '0.4rem' }}>
              模型选择
            </label>
            <select
              value={defaultModel?.id || ''}
              onChange={e => handleModelChange(e.target.value)}
              style={{
                width: '100%', padding: '0.6rem 0.8rem', fontSize: '0.9rem',
                backgroundColor: '#0a0a1a', color: '#e0e0e0',
                border: '1px solid #2a2a3a', borderRadius: '8px', outline: 'none',
                boxSizing: 'border-box',
              }}
            >
              {editedProvider.models.map(model => (
                <option key={model.id} value={model.id}>
                  {model.name}{model.freeQuota ? ` — ${model.freeQuota}` : ''}
                </option>
              ))}
            </select>
          </div>
        )}

        {/* Check Result */}
        {checkResult && (
          <div style={{
            marginBottom: '1.25rem', padding: '0.75rem',
            backgroundColor: checkResult.status === 'ok' ? 'rgba(46,125,50,0.1)' : 'rgba(224,96,96,0.1)',
            border: `1px solid ${checkResult.status === 'ok' ? 'rgba(46,125,50,0.3)' : 'rgba(224,96,96,0.3)'}`,
            borderRadius: '8px', fontSize: '0.85rem',
            color: checkResult.status === 'ok' ? '#a5d6a7' : '#e06060',
          }}>
            {checkResult.status === 'ok' ? '连接成功！' : `连接失败：${checkResult.message || '未知错误'}`}
          </div>
        )}

        {/* Free Quota Info */}
        {defaultModel?.freeQuota && (
          <div style={{
            marginBottom: '1.25rem', padding: '0.75rem',
            backgroundColor: 'rgba(46,125,50,0.1)', border: '1px solid rgba(46,125,50,0.3)',
            borderRadius: '8px', fontSize: '0.85rem', color: '#a5d6a7',
          }}>
            免费额度：{defaultModel.freeQuota}
          </div>
        )}

        {/* Registration Guide */}
        <div style={{
          marginBottom: '1.5rem', padding: '0.75rem',
          backgroundColor: '#0a0a1a', borderRadius: '8px', fontSize: '0.85rem',
        }}>
          <div style={{ color: '#9999bb', marginBottom: '0.5rem' }}>注册指引</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.3rem' }}>
            {editedProvider.officialUrl && (
              <a href={editedProvider.officialUrl} target="_blank" rel="noopener noreferrer"
                style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.8rem' }}>官方网站 →</a>
            )}
            {editedProvider.registerUrl && (
              <a href={editedProvider.registerUrl} target="_blank" rel="noopener noreferrer"
                style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.8rem' }}>注册账号 →</a>
            )}
            {editedProvider.docsUrl && (
              <a href={editedProvider.docsUrl} target="_blank" rel="noopener noreferrer"
                style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.8rem' }}>API 文档 →</a>
            )}
          </div>
        </div>

        {/* Actions */}
        <div style={{ display: 'flex', gap: '0.75rem', justifyContent: 'flex-end' }}>
          <button className="btn btn-secondary" onClick={onClose}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}>
            取消
          </button>
          <button className="btn btn-secondary" onClick={handleCheck}
            disabled={checking || saving}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}>
            {checking ? '检测中...' : saving ? '保存中...' : '测试连接'}
          </button>
          <button className="btn btn-primary" onClick={handleSave}
            disabled={saving}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}>
            {saving ? '保存中...' : '保存'}
          </button>
        </div>
      </div>
    </div>
  );
}
