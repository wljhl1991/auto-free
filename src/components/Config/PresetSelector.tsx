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
        color: '#e8eaed',
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
                border: isActive ? '2px solid #c9a962' : '1px solid #2a3a4e',
                backgroundColor: isActive ? 'rgba(201, 169, 98, 0.08)' : 'rgba(26, 35, 50, 0.9)',
                cursor: 'pointer',
                transition: 'all 0.2s ease',
                position: 'relative',
              }}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.borderColor = '#c9a962';
                  e.currentTarget.style.backgroundColor = 'rgba(42, 58, 78, 0.9)';
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.borderColor = '#2a3a4e';
                  e.currentTarget.style.backgroundColor = 'rgba(26, 35, 50, 0.9)';
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
                  color: isActive ? '#c9a962' : '#e8eaed',
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
                color: '#7a8594',
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
                color: '#5a6577',
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
