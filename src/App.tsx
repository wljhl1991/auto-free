import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import "./App.css";

function App() {
  const [greeting, setGreeting] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    const result = await invoke<string>("greet", { name });
    setGreeting(result);
  }

  return (
    <div className="app">
      <h1>AutoFree</h1>
      <div className="card">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="输入名字..."
        />
        <button onClick={greet}>Greet</button>
      </div>
      {greeting && <p className="greeting">{greeting}</p>}
    </div>
  );
}

export default App;
