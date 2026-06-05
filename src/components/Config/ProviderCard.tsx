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
      borderRadius: '14px',
      border: '1px solid rgba(99, 102, 241, 0.12)',
      backgroundColor: 'rgba(255, 255, 255, 0.7)',
      transition: 'border-color 0.2s ease, box-shadow 0.2s ease',
      backdropFilter: 'blur(12px)',
      WebkitBackdropFilter: 'blur(12px)',
      boxShadow: '0 4px 16px rgba(99, 102, 241, 0.08)',
    }}
      onMouseEnter={(e) => {
        e.currentTarget.style.borderColor = '#6366f1';
        e.currentTarget.style.boxShadow = '0 6px 20px rgba(99, 102, 241, 0.15)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.borderColor = 'rgba(99, 102, 241, 0.12)';
        e.currentTarget.style.boxShadow = '0 4px 16px rgba(99, 102, 241, 0.08)';
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
            color: '#1a1a2e',
            marginBottom: '0.25rem',
          }}>
            {provider.name}
          </h4>
          <p style={{
            fontSize: '0.8rem',
            color: '#6b7280',
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
          color: '#9ca3af',
          marginBottom: '0.75rem',
        }}>
          当前模型：<span style={{ color: '#4a4a6a' }}>{defaultModel.name}</span>
        </div>
      )}

      {provider.errorMessage && (
        <div style={{
          fontSize: '0.75rem',
          color: '#ef4444',
          marginBottom: '0.75rem',
          padding: '0.4rem 0.6rem',
          backgroundColor: 'rgba(239, 68, 68, 0.08)',
          borderRadius: '8px',
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
