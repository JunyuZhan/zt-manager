import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type AuthStatus, type Settings } from "../lib/api";

const REGISTER_URL =
  "https://accounts.zerotier.com/realms/zerotier/protocol/openid-connect/registrations" +
  "?client_id=zt-central" +
  "&redirect_uri=https%3A%2F%2Fmy.zerotier.com%2F" +
  "&response_type=code&scope=openid";

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

  // 登录区状态
  const [auth, setAuth] = useState<AuthStatus>({ loggedIn: false });
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [otp, setOtp] = useState("");
  const [needOtp, setNeedOtp] = useState(false);
  const [loggingIn, setLoggingIn] = useState(false);

  async function refreshAuth() {
    try {
      setAuth(await api.authStatus());
    } catch {
      /* 忽略状态查询失败 */
    }
  }

  useEffect(() => {
    api
      .settingsLoad()
      .then(setSettings)
      .catch((e) => setErr(String(e)));
    refreshAuth();
  }, []);

  async function login() {
    setErr(null);
    setMsg(null);
    setLoggingIn(true);
    try {
      const r = await api.authLogin(email, password, needOtp ? otp : undefined);
      if (r?.needOtp) {
        setNeedOtp(true);
        setMsg("该账号开启了两步验证，请输入验证码");
      } else {
        setNeedOtp(false);
        setOtp("");
        setPassword("");
        setMsg(
          `登录成功（${r?.mode === "central" ? "New Central" : "ZeroTier 账号"} · ${r?.email}）`,
        );
        refreshAuth();
      }
    } catch (e: any) {
      setErr(String(e));
    } finally {
      setLoggingIn(false);
    }
  }

  async function logout() {
    setErr(null);
    try {
      api.authLogout();
      await refreshAuth();
      setMsg("已退出登录");
    } catch (e: any) {
      setErr(String(e));
    }
  }

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
        <div className="flex items-center justify-between">
          <h2 className="font-semibold">ZeroTier 账号登录</h2>
          {auth.loggedIn && (
            <span className="text-xs bg-green-100 text-green-700 px-2 py-0.5 rounded">
              {auth.mode === "central" ? "New Central" : "ZeroTier 账号"} ·{" "}
              {auth.email}
            </span>
          )}
        </div>

        {auth.loggedIn ? (
          <div className="flex items-center justify-between">
            <p className="text-sm text-gray-500">
              已登录，Token 与网络管理操作将自动使用该会话（过期前会自动刷新）。
            </p>
            <button
              onClick={logout}
              className="px-3 py-1.5 bg-gray-200 rounded hover:bg-gray-300 text-sm"
            >
              退出登录
            </button>
          </div>
        ) : (
          <div className="space-y-3">
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="账号邮箱"
              className="border rounded px-2 py-1 w-full"
            />
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && login()}
              placeholder="密码"
              className="border rounded px-2 py-1 w-full"
            />
            {needOtp && (
              <input
                value={otp}
                onChange={(e) => setOtp(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && login()}
                placeholder="两步验证码（6 位）"
                className="border rounded px-2 py-1 w-full font-mono"
              />
            )}
            <div className="flex items-center gap-3">
              <button
                onClick={login}
                disabled={loggingIn || !email || !password}
                className="px-4 py-1.5 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-40"
              >
                {loggingIn ? "登录中…" : needOtp ? "验证并登录" : "登录"}
              </button>
              <button
                onClick={() => openUrl(REGISTER_URL)}
                className="text-sm text-blue-600 hover:underline"
              >
                注册账号
              </button>
            </div>
            <p className="text-xs text-gray-400">
              自动适配两种登录方式：先尝试 New Central（central.zerotier.com），
              失败则回退 ZeroTier 账号（accounts.zerotier.com）。凭据仅用于登录，不落盘。
            </p>
          </div>
        )}
      </section>

      <section className="bg-white rounded shadow p-4 space-y-4">
        <div>
          <label className="block text-sm text-gray-500 mb-1">
            Central API Token（备选，未登录会话时使用）
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
