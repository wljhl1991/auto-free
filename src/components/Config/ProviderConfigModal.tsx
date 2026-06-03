import { useState } from 'react';
import type { AIProviderConfig } from '@/types';
import ConnectivityBadge from './ConnectivityBadge';

interface ProviderConfigModalProps {
  provider: AIProviderConfig;
  isOpen: boolean;
  onClose: () => void;
  onSave: (provider: AIProviderConfig) => void;
  onCheck: () => void;
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

  if (!isOpen) return null;

  const handleApiKeyChange = (value: string) => {
    setEditedProvider((prev) => ({
      ...prev,
      authConfig: {
        ...prev.authConfig,
        apiKey: prev.authConfig.apiKey
          ? { ...prev.authConfig.apiKey, value }
          : { value, label: 'API Key', placeholder: '输入 API Key', helpUrl: '' },
      },
    }));
  };

  const handleModelChange = (modelId: string) => {
    setEditedProvider((prev) => ({
      ...prev,
      models: prev.models.map((m) => ({
        ...m,
        isDefault: m.id === modelId,
      })),
    }));
  };

  const handleSave = () => {
    onSave(editedProvider);
    onClose();
  };

  const defaultModel = editedProvider.models.find((m) => m.isDefault) || editedProvider.models[0];

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.7)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div style={{
        backgroundColor: '#16162a',
        border: '1px solid #2a2a3a',
        borderRadius: '12px',
        padding: '2rem',
        width: '90%',
        maxWidth: '520px',
        maxHeight: '85vh',
        overflowY: 'auto',
      }}>
        {/* Header */}
        <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'flex-start',
          marginBottom: '1.5rem',
        }}>
          <div>
            <h3 style={{
              fontSize: '1.2rem',
              fontWeight: 600,
              color: '#e0e0f0',
              marginBottom: '0.25rem',
            }}>
              {editedProvider.name}
            </h3>
            <p style={{ fontSize: '0.85rem', color: '#8888aa' }}>
              {editedProvider.description}
            </p>
          </div>
          <ConnectivityBadge status={editedProvider.status} />
        </div>

        {/* API Key */}
        {editedProvider.authType === 'api_key' && (
          <div style={{ marginBottom: '1.25rem' }}>
            <label style={{
              display: 'block',
              fontSize: '0.9rem',
              color: '#9999bb',
              marginBottom: '0.4rem',
            }}>
              API Key
            </label>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <input
                type={showApiKey ? 'text' : 'password'}
                value={editedProvider.authConfig.apiKey?.value || ''}
                onChange={(e) => handleApiKeyChange(e.target.value)}
                placeholder={editedProvider.authConfig.apiKey?.placeholder || '输入 API Key'}
                style={{
                  flex: 1,
                  padding: '0.6rem 0.8rem',
                  fontSize: '0.9rem',
                  fontFamily: 'monospace',
                  backgroundColor: '#0a0a1a',
                  color: '#e0e0e0',
                  border: '1px solid #2a2a3a',
                  borderRadius: '8px',
                  outline: 'none',
                }}
                onFocus={(e) => {
                  e.currentTarget.style.borderColor = '#4a90d9';
                }}
                onBlur={(e) => {
                  e.currentTarget.style.borderColor = '#2a2a3a';
                }}
              />
              <button
                className="btn btn-secondary"
                style={{ padding: '0.6rem 0.8rem', fontSize: '0.8rem', whiteSpace: 'nowrap' }}
                onClick={() => setShowApiKey(!showApiKey)}
              >
                {showApiKey ? '隐藏' : '显示'}
              </button>
            </div>
            {editedProvider.authConfig.apiKey?.helpUrl && (
              <a
                href={editedProvider.authConfig.apiKey.helpUrl}
                target="_blank"
                rel="noopener noreferrer"
                style={{
                  display: 'inline-block',
                  marginTop: '0.4rem',
                  fontSize: '0.8rem',
                  color: '#4a90d9',
                  textDecoration: 'none',
                }}
              >
                获取 API Key →
              </a>
            )}
          </div>
        )}

        {/* Model Selection */}
        <div style={{ marginBottom: '1.25rem' }}>
          <label style={{
            display: 'block',
            fontSize: '0.9rem',
            color: '#9999bb',
            marginBottom: '0.4rem',
          }}>
            模型选择
          </label>
          <select
            className="form-select"
            value={defaultModel?.id || ''}
            onChange={(e) => handleModelChange(e.target.value)}
            style={{ width: '100%' }}
          >
            {editedProvider.models.map((model) => (
              <option key={model.id} value={model.id}>
                {model.name} {model.freeQuota ? `(免费额度: ${model.freeQuota})` : ''}
              </option>
            ))}
          </select>
        </div>

        {/* Connectivity Status */}
        {editedProvider.lastChecked && (
          <div style={{
            marginBottom: '1.25rem',
            padding: '0.75rem',
            backgroundColor: '#0a0a1a',
            borderRadius: '8px',
            fontSize: '0.85rem',
          }}>
            <div style={{ color: '#9999bb', marginBottom: '0.25rem' }}>
              连通状态：<ConnectivityBadge status={editedProvider.status} />
            </div>
            {editedProvider.errorMessage && (
              <div style={{ color: '#e06060', marginTop: '0.25rem' }}>
                {editedProvider.errorMessage}
              </div>
            )}
            <div style={{ color: '#666680', marginTop: '0.25rem', fontSize: '0.75rem' }}>
              上次检测：{new Date(editedProvider.lastChecked).toLocaleString()}
            </div>
          </div>
        )}

        {/* Free Quota Info */}
        {defaultModel?.freeQuota && (
          <div style={{
            marginBottom: '1.25rem',
            padding: '0.75rem',
            backgroundColor: 'rgba(46, 125, 50, 0.1)',
            border: '1px solid rgba(46, 125, 50, 0.3)',
            borderRadius: '8px',
            fontSize: '0.85rem',
            color: '#a5d6a7',
          }}>
            免费额度：{defaultModel.freeQuota}
          </div>
        )}

        {/* Registration Guide */}
        <div style={{
          marginBottom: '1.5rem',
          padding: '0.75rem',
          backgroundColor: '#0a0a1a',
          borderRadius: '8px',
          fontSize: '0.85rem',
        }}>
          <div style={{ color: '#9999bb', marginBottom: '0.5rem' }}>注册指引</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.3rem' }}>
            {editedProvider.officialUrl && (
              <a
                href={editedProvider.officialUrl}
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.8rem' }}
              >
                官方网站 →
              </a>
            )}
            {editedProvider.registerUrl && (
              <a
                href={editedProvider.registerUrl}
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.8rem' }}
              >
                注册账号 →
              </a>
            )}
            {editedProvider.docsUrl && (
              <a
                href={editedProvider.docsUrl}
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: '#4a90d9', textDecoration: 'none', fontSize: '0.8rem' }}
              >
                API 文档 →
              </a>
            )}
          </div>
        </div>

        {/* Actions */}
        <div style={{
          display: 'flex',
          gap: '0.75rem',
          justifyContent: 'flex-end',
        }}>
          <button
            className="btn btn-secondary"
            onClick={onCheck}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            检测连接
          </button>
          <button
            className="btn btn-secondary"
            onClick={onClose}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            取消
          </button>
          <button
            className="btn btn-primary"
            onClick={handleSave}
            style={{ padding: '0.6rem 1.2rem', fontSize: '0.9rem' }}
          >
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
