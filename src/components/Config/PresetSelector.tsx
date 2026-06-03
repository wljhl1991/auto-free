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
        color: '#c0c0d0',
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
                border: isActive ? '2px solid #4a90d9' : '1px solid #2a2a3a',
                backgroundColor: isActive ? '#1a1a3a' : '#12121f',
                cursor: 'pointer',
                transition: 'all 0.2s ease',
                position: 'relative',
              }}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.borderColor = '#3a3a5a';
                  e.currentTarget.style.backgroundColor = '#16162a';
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.borderColor = '#2a2a3a';
                  e.currentTarget.style.backgroundColor = '#12121f';
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
                  color: isActive ? '#4a90d9' : '#e0e0f0',
                }}>
                  {preset.name}
                </span>
                {isZeroCost && (
                  <span style={{
                    fontSize: '0.7rem',
                    padding: '2px 8px',
                    borderRadius: '10px',
                    backgroundColor: '#2e7d32',
                    color: '#a5d6a7',
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
                    backgroundColor: '#1565c0',
                    color: '#90caf9',
                    fontWeight: 500,
                  }}>
                    推荐入门
                  </span>
                )}
              </div>
              <p style={{
                fontSize: '0.8rem',
                color: '#8888aa',
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
                color: '#666680',
              }}>
                <span>{preset.vendorCount} 个服务商</span>
                <span style={{ color: '#555570' }}>|</span>
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
