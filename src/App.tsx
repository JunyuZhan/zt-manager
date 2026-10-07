import { useState } from "react";
import Dashboard from "./pages/Dashboard";
import Central from "./pages/Central";
import Controller from "./pages/Controller";
import Diagnostics from "./pages/Diagnostics";
import Settings from "./pages/Settings";

type Page = "dashboard" | "central" | "controller" | "diagnostics" | "settings";

const NAV: { key: Page; label: string }[] = [
  { key: "dashboard", label: "仪表盘" },
  { key: "central", label: "Central 网络" },
  { key: "controller", label: "自建控制器" },
  { key: "diagnostics", label: "诊断" },
  { key: "settings", label: "设置" },
];

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  return (
    <div className="flex h-screen bg-gray-100 text-gray-800">
      <aside className="w-56 bg-gray-900 text-gray-100 flex flex-col">
        <div className="px-4 py-4 text-lg font-semibold border-b border-gray-700">
          ZeroTier 管理器
        </div>
        <nav className="flex-1 p-2 space-y-1">
          {NAV.map((n) => (
            <button
              key={n.key}
              onClick={() => setPage(n.key)}
              className={`w-full text-left px-3 py-2 rounded ${
                page === n.key
                  ? "bg-blue-600 text-white"
                  : "hover:bg-gray-700"
              }`}
            >
              {n.label}
            </button>
          ))}
        </nav>
        <div className="px-4 py-3 text-xs text-gray-500">v0.1.0 · Tauri 2 + React</div>
      </aside>
      <main className="flex-1 overflow-auto p-6">
        {page === "dashboard" && <Dashboard />}
        {page === "central" && <Central />}
        {page === "controller" && <Controller />}
        {page === "diagnostics" && <Diagnostics />}
        {page === "settings" && <Settings />}
      </main>
    </div>
  );
}
