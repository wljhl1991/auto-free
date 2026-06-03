import { useNavigate } from 'react-router-dom';

export default function MainMenu() {
  const navigate = useNavigate();

  return (
    <div className="page main-menu">
      <div className="menu-center">
        <h1 className="app-title">AutoFree</h1>
        <p className="app-subtitle">AI 生成式游戏引擎</p>

        <div className="menu-buttons">
          <button className="btn btn-primary" onClick={() => navigate('/create')}>
            🎮 新游戏
          </button>
          <button className="btn btn-secondary" onClick={() => { /* 占位：继续游戏 */ }}>
            📂 继续游戏
          </button>
          <button className="btn btn-secondary" onClick={() => { /* 占位：游戏列表 */ }}>
            📋 游戏列表
          </button>
          <button className="btn btn-secondary" onClick={() => navigate('/settings')}>
            ⚙️ 设置
          </button>
        </div>
      </div>

      <footer className="version-footer">v0.1.0</footer>
    </div>
  );
}
