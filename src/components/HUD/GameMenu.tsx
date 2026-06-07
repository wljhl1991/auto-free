interface GameMenuProps {
  onSave: () => void;
  onLoad: () => void;
  onBackToMenu: () => void;
  onExportGame: () => void;
  onCancelGeneration?: () => void;
  onRepairGame?: () => void;
  isRepairing?: boolean;
  onClose: () => void;
}

export function GameMenu({ onSave, onLoad, onBackToMenu, onExportGame, onCancelGeneration, onRepairGame, isRepairing, onClose }: GameMenuProps) {
  return (
    <div className="hud-overlay" onClick={(e) => e.stopPropagation()}>
      <div className="hud-panel">
        <div className="hud-panel-header">
          <h3>菜单</h3>
          <button className="hud-close-btn" onClick={onClose}>✕</button>
        </div>
        <div className="hud-panel-body">
          <button className="btn btn-secondary hud-menu-btn" onClick={onSave}>存档</button>
          <button className="btn btn-secondary hud-menu-btn" onClick={onLoad}>读档</button>
          <button className="btn btn-secondary hud-menu-btn" onClick={onExportGame}>导出游戏</button>
          {onRepairGame && (
            <button className="btn btn-secondary hud-menu-btn" onClick={onRepairGame} disabled={isRepairing}>
              {isRepairing ? '修复中...' : '🔧 修复游戏资源'}
            </button>
          )}
          {onCancelGeneration && (
            <button className="btn btn-secondary hud-menu-btn" onClick={onCancelGeneration}>取消后续章节生成</button>
          )}
          <button className="btn btn-secondary hud-menu-btn" onClick={onBackToMenu}>返回主菜单</button>
        </div>
      </div>
    </div>
  );
}
