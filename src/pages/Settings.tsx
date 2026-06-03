import { useNavigate } from 'react-router-dom';

export default function Settings() {
  const navigate = useNavigate();

  return (
    <div className="page settings">
      <button className="btn btn-back" onClick={() => navigate('/')}>
        ← 返回
      </button>
      <h2 className="page-title">设置</h2>
      <p className="placeholder-text">（设置界面将在后续节点完善）</p>
    </div>
  );
}
