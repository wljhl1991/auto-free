interface StatsPanelProps {
  stats: Record<string, number>;
  onClose: () => void;
}

export function StatsPanel({ stats, onClose }: StatsPanelProps) {
  const entries = Object.entries(stats);

  return (
    <div className="hud-overlay" onClick={(e) => e.stopPropagation()}>
      <div className="hud-panel">
        <div className="hud-panel-header">
          <h3>角色属性</h3>
          <button className="hud-close-btn" onClick={onClose}>✕</button>
        </div>
        <div className="hud-panel-body">
          {entries.length === 0 ? (
            <p className="hud-empty-text">暂无属性</p>
          ) : (
            <ul className="hud-list">
              {entries.map(([key, value]) => (
                <li key={key} className="hud-list-item">
                  <span className="hud-stat-name">{key}</span>
                  <span className="hud-stat-value">{value}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
