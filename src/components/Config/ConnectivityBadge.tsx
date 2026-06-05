import type { ProviderStatus } from '@/types';

interface ConnectivityBadgeProps {
  status: ProviderStatus;
}

const statusConfig: Record<ProviderStatus, { color: string; label: string }> = {
  connected: { color: '#4caf50', label: '已连接' },
  configured: { color: '#ff9800', label: '已配置' },
  unconfigured: { color: '#9ca3af', label: '未配置' },
  auth_failed: { color: '#ef4444', label: '认证失败' },
  quota_exceeded: { color: '#ff9800', label: '额度不足' },
  network_error: { color: '#ef4444', label: '网络错误' },
  error: { color: '#ef4444', label: '错误' },
};

export default function ConnectivityBadge({ status }: ConnectivityBadgeProps) {
  const config = statusConfig[status] || statusConfig.unconfigured;

  return (
    <span style={{
      display: 'inline-flex',
      alignItems: 'center',
      gap: '6px',
      fontSize: '0.8rem',
      color: '#718096',
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
