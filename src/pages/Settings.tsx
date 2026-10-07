import { useEffect, useState } from "react";
import { api, type Settings } from "../lib/api";

export default function Settings() {
  const [settings, setSettings] = useState<Settings>({
    token: "",
    centralMode: "auto",
    orgId: null,
    legacyBase: null,
    newBase: null,
  });
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    api
      .settingsLoad()
      .then(setSettings)
      .catch((e) => setErr(String(e)));
  }, []);

  async function save() {
    setMsg(null);
    setErr(null);
    try {
      await api.settingsSave(settings);
      setMsg("已保存（token 已用 DPAPI 加密存储）");
    } catch (e: any) {
      setErr(String(e));
    }
  }

  async function detect() {
    setErr(null);
    try {
      const mode = await api.centralDetect();
      setMsg(
        mode === "new"
          ? "探测结果：New Central (Service Account / Bearer)"
          : "探测结果：Legacy Central (token)",
      );
    } catch (e: any) {
      setErr(String(e));
    }
  }

  return (
    <div className="space-y-6 max-w-2xl">
      <h1 className="text-2xl font-bold">设置</h1>
      {msg && (
        <div className="bg-green-100 text-green-700 p-3 rounded text-sm">{msg}</div>
      )}
      {err && (
        <div className="bg-red-100 text-red-700 p-3 rounded text-sm">{err}</div>
      )}

      <section className="bg-white rounded shadow p-4 space-y-4">
        <div>
          <label className="block text-sm text-gray-500 mb-1">
            Central API Token
          </label>
          <input
            type="password"
            value={settings.token}
            onChange={(e) =>
              setSettings((s) => ({ ...s, token: e.target.value }))
            }
            placeholder="粘贴你的 ZeroTier Central token"
            className="border rounded px-2 py-1 w-full font-mono"
          />
        </div>

        <div>
          <label className="block text-sm text-gray-500 mb-1">认证类型</label>
          <select
            value={settings.centralMode}
            onChange={(e) =>
              setSettings((s) => ({
                ...s,
                centralMode: e.target.value as Settings["centralMode"],
              }))
            }
            className="border rounded px-2 py-1 w-full"
          >
            <option value="auto">自动检测（默认）</option>
            <option value="legacy">Legacy（api.zerotier.com）</option>
            <option value="new">New Central（Service Account）</option>
          </select>
        </div>

        <div>
          <label className="block text-sm text-gray-500 mb-1">
            Org ID（仅 New Central 需要）
          </label>
          <input
            value={settings.orgId ?? ""}
            onChange={(e) =>
              setSettings((s) => ({
                ...s,
                orgId: e.target.value || null,
              }))
            }
            placeholder="组织 ID"
            className="border rounded px-2 py-1 w-full font-mono"
          />
        </div>

        <details className="text-sm">
          <summary className="cursor-pointer text-gray-500">高级：自定义 API 地址</summary>
          <div className="mt-2 space-y-3">
            <div>
              <label className="block text-xs text-gray-500 mb-1">
                Legacy base URL（留空用默认）
              </label>
              <input
                value={settings.legacyBase ?? ""}
                onChange={(e) =>
                  setSettings((s) => ({ ...s, legacyBase: e.target.value || null }))
                }
                placeholder="https://api.zerotier.com/api/v1"
                className="border rounded px-2 py-1 w-full font-mono text-xs"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-1">
                New Central base URL（留空用默认）
              </label>
              <input
                value={settings.newBase ?? ""}
                onChange={(e) =>
                  setSettings((s) => ({ ...s, newBase: e.target.value || null }))
                }
                placeholder="https://central.zerotier.com/api/v2"
                className="border rounded px-2 py-1 w-full font-mono text-xs"
              />
            </div>
          </div>
        </details>

        <div className="space-x-2">
          <button
            onClick={save}
            className="px-4 py-1.5 bg-blue-600 text-white rounded hover:bg-blue-700"
          >
            保存
          </button>
          <button
            onClick={detect}
            className="px-4 py-1.5 bg-gray-200 rounded hover:bg-gray-300"
          >
            探测类型
          </button>
        </div>
      </section>

      <p className="text-xs text-gray-400">
        Token 以 Windows DPAPI（当前用户作用域）加密后保存在应用配置目录，不会以明文落盘。
        也可用环境变量占位符配置：{" "}
        <code className="font-mono">ZTM_CENTRAL_TOKEN</code>、
        <code className="font-mono">ZTM_CENTRAL_MODE</code>、
        <code className="font-mono">ZTM_CENTRAL_ORG_ID</code>、
        <code className="font-mono">ZTM_LEGACY_BASE</code>、
        <code className="font-mono">ZTM_NEW_BASE</code>（UI 未填写时生效）。
      </p>
    </div>
  );
}
