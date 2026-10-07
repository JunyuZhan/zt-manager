import { useEffect, useState } from "react";
import { api } from "../lib/api";

export default function Dashboard() {
  const [svc, setSvc] = useState<any>(null);
  const [status, setStatus] = useState<any>(null);
  const [networks, setNetworks] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [joinId, setJoinId] = useState("");
  const [joining, setJoining] = useState(false);

  async function refresh() {
    setLoading(true);
    setErr(null);
    try {
      const [s, st, ns] = await Promise.all([
        api.serviceQuery(),
        api.agentStatus(),
        api.agentNetworks(),
      ]);
      setSvc(s);
      setStatus(st);
      setNetworks(Array.isArray(ns) ? ns : []);
    } catch (e: any) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function control(action: "start" | "stop" | "restart") {
    setErr(null);
    try {
      const r = await api.serviceControl(action);
      setSvc(r);
    } catch (e: any) {
      setErr(String(e));
    }
  }

  async function join() {
    const id = joinId.trim().toLowerCase();
    if (!/^[0-9a-f]{16}$/.test(id)) {
      setErr("网络 ID 必须是 16 位十六进制");
      return;
    }
    setJoining(true);
    setErr(null);
    try {
      await api.agentJoin(id);
      setJoinId("");
      refresh();
    } catch (e: any) {
      setErr(String(e));
    } finally {
      setJoining(false);
    }
  }

  async function leave(nwid: string) {
    if (!window.confirm(`确认离开网络 ${nwid}？`)) return;
    setErr(null);
    try {
      await api.agentLeave(nwid);
      refresh();
    } catch (e: any) {
      setErr(String(e));
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">仪表盘</h1>
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
        <div className="flex items-center justify-between">
          <div>
            <div className="text-sm text-gray-500">ZeroTierOneService</div>
            <div className="text-lg font-semibold">
              {(svc?.state ?? "unknown").toUpperCase()}
              {svc?.pid ? ` (PID ${svc.pid})` : ""}
            </div>
          </div>
          <div className="space-x-2">
            <button
              onClick={() => control("start")}
              className="px-3 py-1.5 bg-green-600 text-white rounded hover:bg-green-700"
            >
              启动
            </button>
            <button
              onClick={() => control("stop")}
              className="px-3 py-1.5 bg-red-600 text-white rounded hover:bg-red-700"
            >
              停止
            </button>
            <button
              onClick={() => control("restart")}
              className="px-3 py-1.5 bg-amber-600 text-white rounded hover:bg-amber-700"
            >
              重启
            </button>
          </div>
        </div>
      </section>

      <section className="bg-white rounded shadow p-4">
        <h2 className="font-semibold mb-2">本机节点</h2>
        <div className="grid grid-cols-3 gap-4 text-sm">
          <div>
            <div className="text-gray-500">节点地址</div>
            <div className="font-mono">{status?.address ?? "-"}</div>
          </div>
          <div>
            <div className="text-gray-500">版本</div>
            <div>{status?.version ?? "-"}</div>
          </div>
          <div>
            <div className="text-gray-500">在线</div>
            <div>{status?.online ? "是" : "否"}</div>
          </div>
        </div>
      </section>

      <section className="bg-white rounded shadow p-4">
        <div className="flex items-center justify-between mb-2">
          <h2 className="font-semibold">已加入网络 ({networks.length})</h2>
          <div className="flex gap-2">
            <input
              value={joinId}
              onChange={(e) => setJoinId(e.target.value)}
              placeholder="16 位网络 ID"
              maxLength={16}
              className="border rounded px-2 py-1 text-sm font-mono w-44"
            />
            <button
              onClick={join}
              disabled={joining}
              className="px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm disabled:opacity-40"
            >
              {joining ? "加入中…" : "加入"}
            </button>
          </div>
        </div>
        {networks.length === 0 && (
          <div className="text-gray-400 text-sm">暂无（可在「自建控制器」页创建并加入）</div>
        )}
        <ul className="divide-y">
          {networks.map((n) => (
            <NetworkRow key={n} nwid={n} onLeave={leave} />
          ))}
        </ul>
      </section>
    </div>
  );
}

function NetworkRow({
  nwid,
  onLeave,
}: {
  nwid: string;
  onLeave: (nwid: string) => void;
}) {
  const [detail, setDetail] = useState<any>(null);
  useEffect(() => {
    api
      .agentNetworkDetail(nwid)
      .then(setDetail)
      .catch(() => setDetail(null));
  }, [nwid]);
  return (
    <li className="py-2 flex items-center justify-between">
      <div>
        <div className="font-mono text-sm">{nwid}</div>
        <div className="text-xs text-gray-500">
          {detail?.name ?? detail?.config?.name ?? ""} · {detail?.status ?? ""}
        </div>
      </div>
      <div className="text-sm space-x-3">
        {/* 本地 /network 无 config.authorized 字段（实测），授权结果体现在 status */}
        {detail?.status === "OK" ? (
          <span className="text-green-600">已授权</span>
        ) : detail?.status ? (
          <span className="text-amber-600">{detail.status}</span>
        ) : (
          <span className="text-gray-400">-</span>
        )}
        <button
          onClick={() => onLeave(nwid)}
          className="text-red-600 hover:underline"
        >
          离开
        </button>
      </div>
    </li>
  );
}
