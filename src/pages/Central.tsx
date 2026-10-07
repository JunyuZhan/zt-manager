import { useEffect, useState } from "react";
import { api } from "../lib/api";

/** Central v1 成员列表归一化：以 nodeId 作为更新用的 mid，authorized 取 config.authorized */
function normMember(m: any): {
  mid: string;
  nodeId: string;
  name: string;
  desc: string;
  authorized: boolean;
  ips: string[];
  physical: string;
  lastOnline: number;
  version: string;
} {
  const nodeId = m.nodeId ?? m.config?.address ?? m.id;
  return {
    mid: nodeId,
    nodeId,
    name: m.name ?? m.config?.name ?? "",
    desc: m.description ?? "",
    authorized: !!m.config?.authorized,
    ips: m.config?.ipAssignments ?? [],
    physical: m.physicalAddress ?? "",
    lastOnline: m.lastOnline ?? 0,
    version: m.clientVersion ?? "",
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

  async function toggleAuth(mid: string, authorized: boolean) {
    if (!expanded) return;
    try {
      // 官方 go-ztcentral AuthorizeMember：{config:{authorized}}（嵌套 config）
      await api.centralUpdateMember(expanded, mid, { config: { authorized } });
      openMembers(expanded);
    } catch (e: any) {
      setErr(String(e));
    }
  }

  async function approveAll() {
    if (!expanded) return;
    const pending = members.filter((m) => !m.authorized);
    if (pending.length === 0) return;
    setBusy(true);
    setErr(null);
    const failed: string[] = [];
    for (const m of pending) {
      try {
        await api.centralUpdateMember(expanded, m.mid, {
          config: { authorized: true },
        });
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

  const pendingCount = members.filter((m) => !m.authorized).length;

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
                  </td>
                  <td>{n.onlineMemberCount ?? "-"}</td>
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
                  暂无网络（请确认已在「设置」页配置有效 token）
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
                    {m.name}
                    {m.desc && (
                      <span className="text-xs text-gray-400"> ({m.desc})</span>
                    )}
                  </td>
                  <td className="font-mono text-xs">{m.ips.join(", ")}</td>
                  <td className="font-mono text-xs">{m.physical}</td>
                  <td className="text-xs">{fmtTime(m.lastOnline)}</td>
                  <td>
                    {m.authorized ? (
                      <span className="text-green-600">已授权</span>
                    ) : (
                      <span className="text-amber-600">未授权</span>
                    )}
                  </td>
                  <td className="text-right">
                    {m.authorized ? (
                      <button
                        onClick={() => toggleAuth(m.mid, false)}
                        className="text-red-600 hover:underline"
                      >
                        取消授权
                      </button>
                    ) : (
                      <button
                        onClick={() => toggleAuth(m.mid, true)}
                        className="text-blue-600 hover:underline"
                      >
                        批准
                      </button>
                    )}
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
        </section>
      )}
    </div>
  );
}
