import { BrowserRouter, Routes, Route } from 'react-router-dom';
import MainMenu from './pages/MainMenu';
import CreateGame from './pages/CreateGame';
import GenerationProgress from './pages/GenerationProgress';
import GamePlay from './pages/GamePlay';
import Settings from './pages/Settings';

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<MainMenu />} />
        <Route path="/create" element={<CreateGame />} />
        <Route path="/generate/:gameId" element={<GenerationProgress />} />
        <Route path="/play/:gameId" element={<GamePlay />} />
        <Route path="/settings" element={<Settings />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
