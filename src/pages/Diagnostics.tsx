import { useEffect, useState } from "react";
import { api } from "../lib/api";

// 判定规则与 zerotier-cli peers 一致（源码 one.cpp `peers` 分支实测对照）：
// - 有 preferred 路径且未 tunneled → DIRECT，否则 RELAY。
// - path.address 形如 `ip/port`（如 35.206.108.191/25996、2001:.../64420），无 type 字段。
function peerLink(p: any): "DIRECT" | "RELAY" {
  const paths = Array.isArray(p.paths) ? p.paths : [];
  const preferred = paths.some((x: any) => x.preferred);
  return preferred && !p.tunneled ? "DIRECT" : "RELAY";
}

function preferredHost(p: any): string {
  const paths = Array.isArray(p.paths) ? p.paths : [];
  const best = paths.find((x: any) => x.preferred) ?? paths[0];
  const addr: string = best?.address ?? "";
  // 剥掉 `/port`，ping 只需要主机部分
  const slash = addr.indexOf("/");
  return slash >= 0 ? addr.slice(0, slash) : addr;
}

export default function Diagnostics() {
  const [peers, setPeers] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [ping, setPing] = useState<Record<string, number>>({});

  async function refresh() {
    setLoading(true);
    setErr(null);
    try {
      const r = await api.agentPeers();
      const arr = Array.isArray(r)
        ? r
        : Array.isArray(r?.peers)
          ? r.peers
          : [];
      setPeers(arr);
    } catch (e: any) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function doPing(host: string) {
    try {
      const r = await api.pingHost(host);
      setPing((p) => ({ ...p, [host]: r.latencyMs }));
    } catch (e: any) {
      setErr(String(e));
    }
  }

  const relayPeers = peers.filter((p) => peerLink(p) === "RELAY");

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">诊断</h1>
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

      {relayPeers.length > 0 && (
        <div className="bg-amber-50 border border-amber-200 text-amber-700 p-3 rounded text-sm">
          异常汇总：{relayPeers.length} 个节点走中继（relay），P2P 打洞失败，延迟通常更高：
          {relayPeers.map((p) => p.address).join("、")}
        </div>
      )}

      <section className="bg-white rounded shadow p-4">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-gray-500">
              <th className="py-1">地址</th>
              <th>角色</th>
              <th>延迟</th>
              <th>链路</th>
              <th>首选路径</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {peers.map((p) => {
              const link = peerLink(p);
              const host = preferredHost(p);
              return (
                <tr key={p.address} className="border-t">
                  <td className="font-mono py-1">{p.address}</td>
                  <td>{p.role ?? "-"}</td>
                  <td>{p.latency ?? "-"} ms</td>
                  <td>
                    <span
                      className={`inline-block px-1.5 py-0.5 rounded text-xs ${
                        link === "RELAY"
                          ? "bg-amber-100 text-amber-700"
                          : "bg-green-100 text-green-700"
                      }`}
                    >
                      {link}
                    </span>
                    {p.tunneled && (
                      <span className="ml-1 text-xs text-amber-600">TCP隧道</span>
                    )}
                  </td>
                  <td className="font-mono text-xs text-gray-600">{host || "-"}</td>
                  <td className="text-right">
                    {host && (
                      <button
                        onClick={() => doPing(host)}
                        className="text-blue-600 hover:underline"
                      >
                        ping {ping[host] != null ? `(${ping[host]}ms)` : ""}
                      </button>
                    )}
                  </td>
                </tr>
              );
            })}
            {peers.length === 0 && (
              <tr>
                <td colSpan={6} className="text-gray-400 py-2">
                  无 peer
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>
    </div>
  );
}
