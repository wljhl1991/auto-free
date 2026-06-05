import type { ConfigPreset } from '@/types';

interface PresetSelectorProps {
  presets: ConfigPreset[];
  activePresetId: string;
  onSelect: (id: string) => void;
}

export default function PresetSelector({ presets, activePresetId, onSelect }: PresetSelectorProps) {
  return (
    <div style={{ marginBottom: '2rem' }}>
      <h3 style={{
        fontSize: '1.1rem',
        fontWeight: 600,
        color: '#1a1a2e',
        marginBottom: '1rem',
      }}>
        预设方案
      </h3>
      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))',
        gap: '1rem',
      }}>
        {presets.map((preset) => {
          const isActive = preset.id === activePresetId;
          const isZeroCost = preset.id === 'zero_cost';
          const isTextOnly = preset.id === 'text_only';

          return (
            <div
              key={preset.id}
              onClick={() => onSelect(preset.id)}
              style={{
                padding: '1.2rem',
                borderRadius: '10px',
                border: isActive ? '2px solid #6366f1' : '1px solid rgba(99, 102, 241, 0.12)',
                backgroundColor: isActive ? 'rgba(99, 102, 241, 0.08)' : 'rgba(255, 255, 255, 0.7)',
                cursor: 'pointer',
                transition: 'all 0.2s ease',
                position: 'relative',
              }}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.borderColor = '#6366f1';
                  e.currentTarget.style.backgroundColor = 'rgba(255, 255, 255, 0.85)';
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.borderColor = 'rgba(99, 102, 241, 0.12)';
                  e.currentTarget.style.backgroundColor = 'rgba(255, 255, 255, 0.7)';
                }
              }}
            >
              <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: '0.5rem',
                marginBottom: '0.5rem',
              }}>
                <span style={{
                  fontSize: '1rem',
                  fontWeight: 600,
                  color: isActive ? '#6366f1' : '#1a1a2e',
                }}>
                  {preset.name}
                </span>
                {isZeroCost && (
                  <span style={{
                    fontSize: '0.7rem',
                    padding: '2px 8px',
                    borderRadius: '10px',
                    backgroundColor: '#16a34a',
                    color: '#fff',
                    fontWeight: 500,
                  }}>
                    推荐体验
                  </span>
                )}
                {isTextOnly && (
                  <span style={{
                    fontSize: '0.7rem',
                    padding: '2px 8px',
                    borderRadius: '10px',
                    backgroundColor: '#3b82f6',
                    color: '#fff',
                    fontWeight: 500,
                  }}>
                    推荐入门
                  </span>
                )}
              </div>
              <p style={{
                fontSize: '0.8rem',
                color: '#6b7280',
                marginBottom: '0.75rem',
                lineHeight: 1.5,
              }}>
                {preset.description}
              </p>
              <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: '0.75rem',
                fontSize: '0.75rem',
                color: '#9ca3af',
              }}>
                <span>{preset.vendorCount} 个服务商</span>
                <span style={{ color: '#d1d5db' }}>|</span>
                <span>
                  {preset.providers.map((p) => p.providerId).join('、')}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
