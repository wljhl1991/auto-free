import type { ProviderStatus } from '@/types';

interface ConnectivityBadgeProps {
  status: ProviderStatus;
}

const statusConfig: Record<ProviderStatus, { color: string; label: string }> = {
  connected: { color: '#4caf50', label: '已连接' },
  configured: { color: '#ff9800', label: '已配置' },
  unconfigured: { color: '#666680', label: '未配置' },
  error: { color: '#e06060', label: '错误' },
};

export default function ConnectivityBadge({ status }: ConnectivityBadgeProps) {
  const config = statusConfig[status] || statusConfig.unconfigured;

  return (
    <span style={{
      display: 'inline-flex',
      alignItems: 'center',
      gap: '6px',
      fontSize: '0.8rem',
      color: '#aaaacc',
    }}>
      <span style={{
        width: '8px',
        height: '8px',
        borderRadius: '50%',
        backgroundColor: config.color,
        flexShrink: 0,
      }} />
      {config.label}
    </span>
  );
}
