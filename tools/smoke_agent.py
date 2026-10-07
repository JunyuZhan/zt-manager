#!/usr/bin/env python3
"""
ZeroTier 本地 agent（127.0.0.1:9993）集成冒烟测试。

用途：验证本应用「仪表盘 / 诊断 / 自建控制器」三页所依赖的本地 API 形状是否正确。
全程只读，不改动本机网络与成员状态。

用法：
    # Windows 需管理员终端（authtoken.secret 的 ACL 仅允许 SYSTEM/管理员读取）
    python tools/smoke_agent.py

    # 也可显式注入（便于 CI 或已自行取得 token 的场景）
    set ZTM_AGENT_TOKEN=xxxxxxxxxxxxxxxxxxxxxxxx
    set ZTM_AGENT_BASE=http://127.0.0.1:9993
    python tools/smoke_agent.py

退出码：0 全部通过；1 有失败项；2 前置条件不满足（未运行 / 无权限）。
"""
import json
import os
import sys
import urllib.error
import urllib.request

BASE = os.environ.get("ZTM_AGENT_BASE", "http://127.0.0.1:9993")

# 各平台 authtoken.secret 的默认位置
CANDIDATES = [
    r"C:\ProgramData\ZeroTier\One\authtoken.secret",
    "/Library/Application Support/ZeroTier/One/authtoken.secret",
    "/var/lib/zerotier-one/authtoken.secret",
]

ok, fail = [], []


def load_token() -> str:
    env = os.environ.get("ZTM_AGENT_TOKEN", "").strip()
    if env:
        return env
    for p in CANDIDATES:
        try:
            with open(p, encoding="utf-8") as f:
                t = f.read().strip()
            if t:
                return t
        except OSError:
            continue
    return ""


def call(path: str):
    req = urllib.request.Request(
        f"{BASE}{path}",
        headers={"X-ZT1-Auth": TOKEN, "User-Agent": "zt-manager-smoke/1.0"},
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as f:
            raw = f.read().decode()
            return f.status, (json.loads(raw) if raw.strip() else {})
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()[:200]
    except Exception as e:  # noqa: BLE001
        return 0, str(e)


def check(name, cond, detail=""):
    (ok if cond else fail).append(name)
    print(f"  [{'PASS' if cond else 'FAIL'}] {name}" + (f"  {detail}" if detail else ""))


print("== 0. 前置条件 ==")
TOKEN = load_token()
if not TOKEN:
    sys.exit(
        "未取得 authtoken：未设置 ZTM_AGENT_TOKEN，且默认路径均不可读。\n"
        "（Windows 上 authtoken.secret 的 ACL 仅允许 SYSTEM/管理员读取，"
        "请用管理员终端运行本脚本。）"
    )
st, _ = call("/status")
if st == 0:
    sys.exit("无法连接 127.0.0.1:9993：ZeroTier 服务可能未运行。")
st, status = call("/status")
if st == 401 or status == {}:
    sys.exit("收到 401：token 无效或未被服务端接受。")
check("本地 agent 可访问", st == 200, f"HTTP {st}")

print("== 1. /status（仪表盘：节点信息）==")
check("返回 200", st == 200, f"HTTP {st}")
check("含 address（10 位节点 ID）", isinstance(status.get("address"), str)
      and len(status.get("address", "")) == 10, str(status.get("address")))
check("含 version", "version" in status, str(status.get("version")))
check("含 online 或 tcpFallbackRelay 等运行态字段",
      any(k in status for k in ("online", "tcpFallbackRelay", "publicIdentity")),
      ",".join(sorted(status.keys())[:8]))

print("== 2. /network（仪表盘：已加入网络）==")
st, nets = call("/network")
check("返回 200", st == 200, f"HTTP {st}")
check("是数组（agent.rs::network_list 按字符串数组解析）", isinstance(nets, list),
      type(nets).__name__)
if isinstance(nets, list) and nets:
    check("元素均为网络 ID 字符串", all(isinstance(x, str) for x in nets),
          f"{len(nets)} 个：{','.join(nets[:3])}")
    print(f"  已加入网络 {len(nets)} 个：")
    for nwid in nets[:5]:
        st_d, d = call(f"/network/{nwid}")
        name = (d or {}).get("portDeviceName") if isinstance(d, dict) else None
        addrs = (d or {}).get("assignedAddresses") if isinstance(d, dict) else None
        check(f"  /network/{nwid} 详情 200", st_d == 200, f"HTTP {st_d}")
        print(f"    {nwid}  设备名={name}  分配地址={addrs}")
    if not nets:
        print("  [SKIP] 本机未加入任何网络，跳过详情校验")
else:
    print("  [SKIP] 本机未加入任何网络")

print("== 3. /peer（诊断页：对端与链路质量）==")
st, peers = call("/peer")
check("返回 200", st == 200, f"HTTP {st}")
check("是数组", isinstance(peers, list), type(peers).__name__)
if isinstance(peers, list) and peers:
    p0 = peers[0]
    check("peer 含 address", "address" in p0)
    check("peer 含 latency（诊断页展示延迟）", "latency" in p0)
    check("peer 含 role（诊断页据此区分 LEAF/ROOT）", "role" in p0)
    check("peer 含 paths（判断直连/中继）", "paths" in p0)
    relay = sum(1 for p in peers if not any(
        (pp or {}).get("active") for pp in (p.get("paths") or [])))
    print(f"  对端 {len(peers)} 个，其中无活动直连路径（走中继）{relay} 个")
else:
    print("  [SKIP] 暂无对端")

print("== 4. /controller/network（自建控制器页）==")
st, cnets = call("/controller/network")
check("返回 200", st == 200, f"HTTP {st}")
check("是数组", isinstance(cnets, list), type(cnets).__name__)
if isinstance(cnets, list):
    print(f"  控制器已托管网络 {len(cnets)} 个")
    for nwid in cnets[:3]:
        st_m, mem = call(f"/controller/network/{nwid}/member")
        n = len(mem) if isinstance(mem, dict) else -1
        check(f"  /controller/network/{nwid}/member 200", st_m == 200, f"HTTP {st_m}")
        print(f"    {nwid}  成员 {n} 个")

print("\n== 汇总 ==")
print(f"  通过 {len(ok)} / 失败 {len(fail)}")
if fail:
    print("  失败项： " + "、".join(fail))
    sys.exit(1)
print("  全部通过：本地 agent 的响应形状与本应用实现一致。")
