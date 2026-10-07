import { useEffect, useState } from "react";
import { api } from "../lib/api";

function toMemberList(m: any): any[] {
  if (!m) return [];
  if (Array.isArray(m)) return m.map((x: any) => ({ id: x.id ?? x.address, ...x }));
  return Object.entries(m).map(([id, v]) => ({ id, ...(v as any) }));
}

export default function Controller() {
  const [networks, setNetworks] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [detail, setDetail] = useState<any>(null);
  const [members, setMembers] = useState<any>(null);
  const [editPools, setEditPools] = useState("[]");
  const [editRoutes, setEditRoutes] = useState("[]");
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setErr(null);
    try {
      const r = await api.controllerNetworks();
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

  async function open(nwid: string) {
    setExpanded(nwid);
    setDetail(null);
    setMembers(null);
    try {
      const d = await api.controllerNetwork(nwid);
      setDetail(d);
      setEditPools(JSON.stringify(d?.ipAssignmentPools ?? [], null, 2));
      setEditRoutes(JSON.stringify(d?.routes ?? [], null, 2));
    } catch (e: any) {
      setErr(String(e));
    }
    try {
      setMembers(await api.controllerMembers(nwid));
    } catch {
      /* ignore */
    }
  }

  async function setAuth(mid: string, authorized: boolean) {
    if (!expanded) return;
    try {
      await api.controllerUpdateMember(expanded, mid, { authorized });
      open(expanded);
    } catch (e: any) {
      setErr(String(e));
    }
  }

  /** 保存 IP 池与路由：本地 controller 的更新接口要求提交完整网络对象 */
  async function saveConfig() {
    if (!expanded || !detail) return;
    let pools: unknown;
    let routes: unknown;
    try {
      pools = JSON.parse(editPools);
      routes = JSON.parse(editRoutes);
    } catch {
      setErr("IP 池或路由不是合法 JSON，已取消保存");
      return;
    }
    setSaving(true);
    setErr(null);
    try {
      const body = { ...detail, ipAssignmentPools: pools, routes };
      await api.controllerUpdate(expanded, body);
      await open(expanded);
      setMsg("已保存 IP 池与路由");
    } catch (e: any) {
      setErr(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function create() {
    try {
      await api.controllerCreate();
      refresh();
    } catch (e: any) {
      setErr(String(e));
    }
  }

  const list = toMemberList(members);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">自建控制器</h1>
        <div className="space-x-2">
          <button
            onClick={create}
            className="px-3 py-1.5 bg-blue-600 text-white rounded hover:bg-blue-700"
          >
            新建网络
          </button>
          <button
            onClick={refresh}
            className="px-3 py-1.5 bg-gray-200 rounded hover:bg-gray-300"
          >
            {loading ? "刷新中…" : "刷新"}
          </button>
        </div>
      </div>
      {err && (
        <div className="bg-red-100 text-red-700 p-3 rounded text-sm">{err}</div>
      )}

      <section className="bg-white rounded shadow p-4">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-gray-500">
              <th className="py-1">网络 ID</th>
              <th>名称</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {networks.map((n) => (
              <tr key={n} className="border-t">
                <td className="font-mono py-1">{n}</td>
                <td>{detail && expanded === n ? detail.name : ""}</td>
                <td className="text-right">
                  <button
                    onClick={() => open(n)}
                    className="text-blue-600 hover:underline"
                  >
                    管理
                  </button>
                </td>
              </tr>
            ))}
            {networks.length === 0 && (
              <tr>
                <td colSpan={3} className="text-gray-400 py-2">
                  本机未作为控制器托管网络（新建将随机生成网络 ID）
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>

      {expanded && (
        <section className="bg-white rounded shadow p-4 space-y-4">
          <h2 className="font-semibold">网络 · {expanded}</h2>

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
              disabled={saving}
              className="px-4 py-1.5 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-40"
            >
              {saving ? "保存中…" : "保存 IP 池与路由"}
            </button>
            {msg && <span className="text-sm text-green-700">{msg}</span>}
          </div>
          <div>
            <h3 className="text-sm text-gray-500 mb-1">成员</h3>
            <table className="w-full text-sm">
              <tbody>
                {list.map((m) => (
                  <tr key={m.id} className="border-t">
                    <td className="font-mono py-1">{m.id}</td>
                    <td>{m.name ?? m.config?.name ?? ""}</td>
                    <td className="text-right space-x-2">
                      {m.authorized ? (
                        <button
                          onClick={() => setAuth(m.id, false)}
                          className="text-red-600 hover:underline"
                        >
                          取消授权
                        </button>
                      ) : (
                        <button
                          onClick={() => setAuth(m.id, true)}
                          className="text-green-600 hover:underline"
                        >
                          授权
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
                {list.length === 0 && (
                  <tr>
                    <td colSpan={3} className="text-gray-400 py-2">
                      无成员
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </section>
      )}
    </div>
  );
}
