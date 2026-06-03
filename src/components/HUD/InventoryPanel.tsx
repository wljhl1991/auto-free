interface InventoryPanelProps {
  items: string[];
  onClose: () => void;
}

export function InventoryPanel({ items, onClose }: InventoryPanelProps) {
  return (
    <div className="hud-overlay" onClick={(e) => e.stopPropagation()}>
      <div className="hud-panel">
        <div className="hud-panel-header">
          <h3>物品栏</h3>
          <button className="hud-close-btn" onClick={onClose}>✕</button>
        </div>
        <div className="hud-panel-body">
          {items.length === 0 ? (
            <p className="hud-empty-text">暂无物品</p>
          ) : (
            <ul className="hud-list">
              {items.map((item, index) => (
                <li key={index} className="hud-list-item">{item}</li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
