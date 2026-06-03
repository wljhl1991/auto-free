import type { AIProviderConfig } from '@/types';
import ConnectivityBadge from './ConnectivityBadge';

interface ProviderCardProps {
  provider: AIProviderConfig;
  onConfigure: () => void;
  onCheck: () => void;
}

export default function ProviderCard({ provider, onConfigure, onCheck }: ProviderCardProps) {
  const defaultModel = provider.models.find((m) => m.isDefault) || provider.models[0];

  return (
    <div style={{
      padding: '1.2rem',
      borderRadius: '10px',
      border: '1px solid #2a2a3a',
      backgroundColor: '#12121f',
      transition: 'border-color 0.2s ease',
    }}
      onMouseEnter={(e) => {
        e.currentTarget.style.borderColor = '#3a3a5a';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.borderColor = '#2a2a3a';
      }}
    >
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'flex-start',
        marginBottom: '0.5rem',
      }}>
        <div>
          <h4 style={{
            fontSize: '1rem',
            fontWeight: 600,
            color: '#e0e0f0',
            marginBottom: '0.25rem',
          }}>
            {provider.name}
          </h4>
          <p style={{
            fontSize: '0.8rem',
            color: '#8888aa',
            lineHeight: 1.4,
          }}>
            {provider.description}
          </p>
        </div>
        <ConnectivityBadge status={provider.status} />
      </div>

      {defaultModel && (
        <div style={{
          fontSize: '0.8rem',
          color: '#666680',
          marginBottom: '0.75rem',
        }}>
          当前模型：<span style={{ color: '#9999bb' }}>{defaultModel.name}</span>
        </div>
      )}

      {provider.errorMessage && (
        <div style={{
          fontSize: '0.75rem',
          color: '#e06060',
          marginBottom: '0.75rem',
          padding: '0.4rem 0.6rem',
          backgroundColor: 'rgba(224, 96, 96, 0.1)',
          borderRadius: '6px',
        }}>
          {provider.errorMessage}
        </div>
      )}

      <div style={{
        display: 'flex',
        gap: '0.5rem',
      }}>
        <button
          className="btn btn-secondary"
          style={{ padding: '0.4rem 1rem', fontSize: '0.85rem' }}
          onClick={onConfigure}
        >
          配置
        </button>
        <button
          className="btn btn-secondary"
          style={{ padding: '0.4rem 1rem', fontSize: '0.85rem' }}
          onClick={onCheck}
        >
          检测连接
        </button>
      </div>
    </div>
  );
}
