import { useEffect, useState } from "react";
import { api, type AuthStatus, type MemberAction } from "../lib/api";

/** 成员归一化（后端已输出统一形状，此处仅做缺省兜底） */
function normMember(m: any): {
  mid: string;
  nodeId: string;
  name: string;
  desc: string;
  authorized: boolean;
  status: string;
  ips: string[];
  physical: string;
  lastOnline: number;
  online: boolean;
  version: string;
  os: string;
  lastSeenIp: string;
} {
  const nodeId = m.nodeId ?? m.mid ?? m.config?.address ?? m.id;
  const authorized = !!m.authorized;
  return {
    mid: m.mid ?? nodeId,
    nodeId,
    name: m.name ?? "",
    desc: m.desc ?? m.description ?? "",
    authorized,
    status: m.status ?? (authorized ? "authorized" : "not_authorized"),
    ips: m.ips ?? [],
    physical: m.physical ?? "",
    lastOnline: m.lastOnline ?? 0,
    online: !!m.online,
    version: m.version ?? "",
    os: m.os ?? "",
    lastSeenIp: m.lastSeenIp ?? "",
  };
}

function fmtTime(ms: number): string {
  if (!ms) return "从未";
  const d = new Date(ms);
  return d.toLocaleString();
}

export default function Central() {
  const [networks, setNetworks] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [members, setMembers] = useState<any[]>([]);
  const [newName, setNewName] = useState("");
  const [busy, setBusy] = useState(false);
  const [editPools, setEditPools] = useState("[]");
  const [editRoutes, setEditRoutes] = useState("[]");
  const [cfgMsg, setCfgMsg] = useState<string | null>(null);
  const [cfgNwid, setCfgNwid] = useState<string | null>(null);
  const [cfgName, setCfgName] = useState("");
  const [savingCfg, setSavingCfg] = useState(false);
  const [auth, setAuth] = useState<AuthStatus>({ loggedIn: false });
  const [editMember, setEditMember] = useState<any | null>(null);
  const [editName, setEditName] = useState("");
  const [editDesc, setEditDesc] = useState("");
  const [editIps, setEditIps] = useState("");
  const [savingMember, setSavingMember] = useState(false);

  /** 未登录且未配置 token 时展示登录引导 */
  const needLogin = !auth.loggedIn && !auth.tokenConfigured;

  async function refresh() {
    setLoading(true);
    setErr(null);
    try {
      const r = await api.centralNetworks();
      setNetworks(Array.isArray(r) ? r : []);
    } catch (e: any) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    api.authStatus().then(setAuth).catch(() => {});
    refresh();
  }, []);

  async function openMembers(nwid: string) {
    setExpanded(nwid);
    setMembers([]);
    try {
      const raw = await api.centralMembers(nwid);
      const arr = Array.isArray(raw) ? raw : [];
      setMembers(arr.map(normMember));
    } catch (e: any) {
      setErr(String(e));
    }
  }

  /** 成员授权动作：authorize / deauthorize / reject */
  async function memberAction(mid: string, action: MemberAction) {
    if (!expanded) return;
    try {
      await api.centralMemberAction(expanded, mid, action);
      openMembers(expanded);
    } catch (e: any) {
      setErr(String(e));
    }
  }

  async function removeMember(m: any) {
    if (!expanded) return;
    const label = m.name || m.nodeId;
    if (!window.confirm(`确认将设备「${label}」从网络中删除？`)) return;
    try {
      await api.centralDeleteMember(expanded, m.mid);
      openMembers(expanded);
    } catch (e: any) {
      setErr(String(e));
    }
  }

  /** 打开设备编辑（名称 / 描述 / IP） */
  function openEditMember(m: any) {
    setEditMember(m);
    setEditName(m.name);
    setEditDesc(m.desc);
    setEditIps(m.ips.join(", "));
  }

  async function saveEditMember() {
    if (!expanded || !editMember) return;
    setSavingMember(true);
    setErr(null);
    try {
      const ips = editIps
        .split(/[\s,]+/)
        .map((s) => s.trim())
        .filter(Boolean);
      await api.centralUpdateMember(expanded, editMember.mid, {
        name: editName,
        description: editDesc,
        ipAssignments: ips,
      });
      setEditMember(null);
      openMembers(expanded);
    } catch (e: any) {
      setErr(String(e));
    } finally {
      setSavingMember(false);
    }
  }

  async function approveAll() {
    if (!expanded) return;
    const pending = members.filter((m) => !m.authorized && m.status !== "rejected");
    if (pending.length === 0) return;
    setBusy(true);
    setErr(null);
    const failed: string[] = [];
    for (const m of pending) {
      try {
        await api.centralMemberAction(expanded, m.mid, "authorize");
      } catch {
        failed.push(m.mid);
      }
    }
    setBusy(false);
    if (failed.length) setErr(`以下成员批准失败：${failed.join("、")}`);
    openMembers(expanded);
  }

  async function createNetwork() {
    if (!newName.trim()) return;
    try {
      await api.centralCreateNetwork(newName.trim());
      setNewName("");
      refresh();
    } catch (e: any) {
      setErr(String(e));
    }
  }

  async function rename(nwid: string, old: string) {
    const name = window.prompt("新名称", old);
    if (name == null) return;
    try {
      // 官方 go-ztcentral UpdateNetwork：名称嵌套在 config 下
      await api.centralUpdateNetwork(nwid, { config: { name } });
      refresh();
    } catch (e: any) {
      setErr(String(e));
    }
  }

  /** 打开网络配置编辑（IP 池 / 路由 / 名称） */
  async function openConfig(nwid: string) {
    setCfgNwid(nwid);
    setCfgMsg(null);
    setErr(null);
    try {
      const n = await api.centralNetwork(nwid);
      const c = n?.config ?? {};
      setCfgName(c.name ?? "");
      setEditPools(JSON.stringify(c.ipAssignmentPools ?? [], null, 2));
      setEditRoutes(JSON.stringify(c.routes ?? [], null, 2));
    } catch (e: any) {
      setErr(String(e));
    }
  }

  /** 保存配置：可写字段全部嵌套在 config 下（官方 go-ztcentral UpdateNetwork 形状） */
  async function saveConfig() {
    if (!cfgNwid) return;
    let pools: unknown;
    let routes: unknown;
    try {
      pools = JSON.parse(editPools);
      routes = JSON.parse(editRoutes);
    } catch {
      setErr("IP 池或路由不是合法 JSON，已取消保存");
      return;
    }
    setSavingCfg(true);
    setErr(null);
    setCfgMsg(null);
    try {
      await api.centralUpdateNetwork(cfgNwid, {
        config: { name: cfgName, ipAssignmentPools: pools, routes },
      });
      setCfgMsg("已保存");
      refresh();
    } catch (e: any) {
      setErr(String(e));
    } finally {
      setSavingCfg(false);
    }
  }

  async function del(nwid: string, name: string) {
    if (!window.confirm(`确认删除网络「${name || nwid}」？此操作不可撤销。`)) return;
    try {
      await api.centralDeleteNetwork(nwid);
      if (expanded === nwid) setExpanded(null);
      refresh();
    } catch (e: any) {
      setErr(String(e));
    }
  }

  const pendingCount = members.filter(
    (m) => !m.authorized && m.status !== "rejected",
  ).length;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Central 网络</h1>
        <button
          onClick={refresh}
          className="px-3 py-1.5 bg-gray-200 rounded hover:bg-gray-300"
        >
          {loading ? "刷新中…" : "刷新"}
        </button>
      </div>
      {err && (
        <div className="bg-red-100 text-red-700 p-3 rounded text-sm">{err}</div>
      )}
      {needLogin && (
        <div className="bg-amber-50 border border-amber-200 text-amber-800 p-3 rounded text-sm">
          尚未登录：请到「设置」页使用 ZeroTier 账号登录（或粘贴 API Token）后再管理网络与设备。
        </div>
      )}

      <section className="bg-white rounded shadow p-4">
        <h2 className="font-semibold mb-2">网络列表</h2>
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-gray-500">
              <th className="py-1">网络 ID</th>
              <th>名称</th>
              <th>成员</th>
              <th>在线</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {networks.map((n: any) => {
              const name = n.config?.name ?? n.name ?? "";
              return (
                <tr key={n.id} className="border-t">
                  <td className="font-mono py-1">{n.id}</td>
                  <td>{name}</td>
                  <td>
                    {n.authorizedMemberCount ?? "-"} / {n.totalMemberCount ?? "-"}
                    {n.pendingMemberCount > 0 && (
                      <span className="ml-1 text-xs text-amber-600">
                        （{n.pendingMemberCount} 待授权）
                      </span>
                    )}
                  </td>
                  <td>
                    {n.onlineMemberCount ?? "-"}
                    {n.onlineMemberCount > 0 && (
                      <span className="ml-1 inline-block w-2 h-2 rounded-full bg-green-500 align-middle" />
                    )}
                  </td>
                  <td className="space-x-2 text-right whitespace-nowrap">
                    <button
                      onClick={() => openMembers(n.id)}
                      className="text-blue-600 hover:underline"
                    >
                      成员
                    </button>
                    <button
                      onClick={() => openConfig(n.id)}
                      className="text-gray-600 hover:underline"
                    >
                      配置
                    </button>
                    <button
                      onClick={() => rename(n.id, name)}
                      className="text-gray-600 hover:underline"
                    >
                      改名
                    </button>
                    <button
                      onClick={() => del(n.id, name)}
                      className="text-red-600 hover:underline"
                    >
                      删除
                    </button>
                  </td>
                </tr>
              );
            })}
            {networks.length === 0 && (
              <tr>
                <td colSpan={5} className="text-gray-400 py-2">
                  暂无网络（请先在「设置」页登录 ZeroTier 账号或配置 Token）
                </td>
              </tr>
            )}
          </tbody>
        </table>

        <div className="flex gap-2 mt-4 pt-4 border-t">
          <input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="新网络名称"
            className="border rounded px-2 py-1 flex-1"
          />
          <button
            onClick={createNetwork}
            className="px-3 py-1.5 bg-blue-600 text-white rounded hover:bg-blue-700"
          >
            新建网络
          </button>
        </div>
      </section>

      {cfgNwid && (
        <section className="bg-white rounded shadow p-4 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="font-semibold">
              网络配置 · <span className="font-mono">{cfgNwid}</span>
            </h2>
            <button
              onClick={() => setCfgNwid(null)}
              className="text-gray-500 hover:text-gray-700 text-sm"
            >
              收起
            </button>
          </div>
          <div>
            <h3 className="text-sm text-gray-500 mb-1">名称</h3>
            <input
              value={cfgName}
              onChange={(e) => setCfgName(e.target.value)}
              className="border rounded px-2 py-1 text-sm w-full"
            />
          </div>
          <div>
            <h3 className="text-sm text-gray-500 mb-1">
              IP 分配池（JSON，可编辑）
            </h3>
            <textarea
              value={editPools}
              onChange={(e) => setEditPools(e.target.value)}
              rows={5}
              className="w-full font-mono text-xs border rounded p-2"
            />
          </div>
          <div>
            <h3 className="text-sm text-gray-500 mb-1">路由（JSON，可编辑）</h3>
            <textarea
              value={editRoutes}
              onChange={(e) => setEditRoutes(e.target.value)}
              rows={5}
              className="w-full font-mono text-xs border rounded p-2"
            />
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={saveConfig}
              disabled={savingCfg}
              className="px-4 py-1.5 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-40"
            >
              {savingCfg ? "保存中…" : "保存配置"}
            </button>
            {cfgMsg && <span className="text-sm text-green-700">{cfgMsg}</span>}
          </div>
        </section>
      )}

      {expanded && (
        <section className="bg-white rounded shadow p-4">
          <div className="flex items-center justify-between mb-2">
            <h2 className="font-semibold">
              成员 · <span className="font-mono">{expanded}</span>
              {pendingCount > 0 && (
                <span className="ml-2 text-xs bg-amber-100 text-amber-700 px-2 py-0.5 rounded">
                  {pendingCount} 个待批准
                </span>
              )}
            </h2>
            <button
              onClick={approveAll}
              disabled={busy || pendingCount === 0}
              className="px-3 py-1.5 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-40"
            >
              {busy ? "处理中…" : "一键批准全部未授权"}
            </button>
          </div>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-gray-500">
                <th className="py-1">节点 ID</th>
                <th>名称</th>
                <th>IP</th>
                <th>物理地址</th>
                <th>最后在线</th>
                <th>状态</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {members.map((m) => (
                <tr key={m.mid} className="border-t">
                  <td className="font-mono py-1">{m.nodeId}</td>
                  <td>
                    {m.name || <span className="text-gray-400">（未命名）</span>}
                    {m.desc && (
                      <span className="text-xs text-gray-400"> ({m.desc})</span>
                    )}
                  </td>
                  <td className="font-mono text-xs">{m.ips.join(", ")}</td>
                  <td className="font-mono text-xs">{m.physical}</td>
                  <td className="text-xs">
                    {m.online && (
                      <span className="inline-block w-2 h-2 rounded-full bg-green-500 mr-1 align-middle" />
                    )}
                    {fmtTime(m.lastOnline)}
                    {m.lastSeenIp && (
                      <span className="block text-gray-400">{m.lastSeenIp}</span>
                    )}
                  </td>
                  <td>
                    {m.status === "authorized" && (
                      <span className="text-green-600">已授权</span>
                    )}
                    {m.status === "not_authorized" && (
                      <span className="text-amber-600">未授权</span>
                    )}
                    {m.status === "rejected" && (
                      <span className="text-red-600">已拒绝</span>
                    )}
                  </td>
                  <td className="text-right space-x-2 whitespace-nowrap">
                    {m.status === "not_authorized" ? (
                      <>
                        <button
                          onClick={() => memberAction(m.mid, "authorize")}
                          className="text-blue-600 hover:underline"
                        >
                          批准
                        </button>
                        <button
                          onClick={() => memberAction(m.mid, "reject")}
                          className="text-red-600 hover:underline"
                        >
                          拒绝
                        </button>
                      </>
                    ) : m.authorized ? (
                      <button
                        onClick={() => memberAction(m.mid, "deauthorize")}
                        className="text-red-600 hover:underline"
                      >
                        取消授权
                      </button>
                    ) : (
                      <button
                        onClick={() => memberAction(m.mid, "authorize")}
                        className="text-blue-600 hover:underline"
                      >
                        恢复授权
                      </button>
                    )}
                    <button
                      onClick={() => openEditMember(m)}
                      className="text-gray-600 hover:underline"
                    >
                      编辑
                    </button>
                    <button
                      onClick={() => removeMember(m)}
                      className="text-red-600 hover:underline"
                    >
                      删除
                    </button>
                  </td>
                </tr>
              ))}
              {members.length === 0 && (
                <tr>
                  <td colSpan={7} className="text-gray-400 py-2">
                    无成员
                  </td>
                </tr>
              )}
            </tbody>
          </table>

          {editMember && (
            <div className="mt-4 pt-4 border-t space-y-3">
              <h3 className="text-sm font-semibold">
                编辑设备 · <span className="font-mono">{editMember.nodeId}</span>
              </h3>
              <div className="grid grid-cols-3 gap-3">
                <div>
                  <label className="block text-xs text-gray-500 mb-1">名称</label>
                  <input
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    className="border rounded px-2 py-1 text-sm w-full"
                  />
                </div>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">描述</label>
                  <input
                    value={editDesc}
                    onChange={(e) => setEditDesc(e.target.value)}
                    className="border rounded px-2 py-1 text-sm w-full"
                  />
                </div>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">
                    IP（逗号分隔）
                  </label>
                  <input
                    value={editIps}
                    onChange={(e) => setEditIps(e.target.value)}
                    className="border rounded px-2 py-1 text-sm w-full font-mono"
                  />
                </div>
              </div>
              <div className="space-x-2">
                <button
                  onClick={saveEditMember}
                  disabled={savingMember}
                  className="px-4 py-1.5 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-40"
                >
                  {savingMember ? "保存中…" : "保存"}
                </button>
                <button
                  onClick={() => setEditMember(null)}
                  className="px-4 py-1.5 bg-gray-200 rounded hover:bg-gray-300"
                >
                  取消
                </button>
              </div>
            </div>
          )}
        </section>
      )}
    </div>
  );
}
