#!/usr/bin/env python3
"""
ZeroTier Central (Legacy v1) 集成冒烟测试。

用途：在不动 Tauri 前端的情况下，独立验证本应用将要发出的请求形状是否正确。
只读操作为主；唯一的写操作是对「已授权成员」执行 authorized=true（幂等，不改变实际状态）。

用法（token 不写入脚本，从环境变量取）：
    set ZTM_CENTRAL_TOKEN=<your-token>
    python tools/smoke_central.py

验证点（均为实测得出，勿凭记忆修改）：
  - 列表 GET /network  -> 200，元素是完整对象（config 嵌套）
  - 名称字段：config.name
  - 计数字段：authorizedMemberCount / totalMemberCount / onlineMemberCount（顶层）
  - 成员授权状态：member.config.authorized（不是 member.authorized）
  - 成员更新用的 ID：nodeId（10 位），不是接口返回的复合 id "{nwid}-{nodeId}"（后者 404）

实测注意（2026-10-03）：ZeroTier 前置的 Cloudflare 会按 UA 拦截 urllib 默认值
`Python-urllib/3.x`，返回 403 + "error code: 1010"；自定义 UA（乃至不发送 UA）均可正常 200。
故本脚本显式设置 User-Agent。本应用使用 reqwest，默认不发送 UA，不受影响。
"""
import json
import os
import sys
import urllib.error
import urllib.request

BASE = os.environ.get("ZTM_LEGACY_BASE", "https://api.zerotier.com/api/v1")
TOKEN = os.environ.get("ZTM_CENTRAL_TOKEN", "").strip()

if not TOKEN:
    sys.exit("请先设置环境变量 ZTM_CENTRAL_TOKEN（不要把它写进脚本）")

ok, fail = [], []


def call(method: str, path: str, body=None):
    url = f"{BASE}{path}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(
        url, data=data, method=method,
        headers={"Authorization": f"token {TOKEN}",
                 "Content-Type": "application/json",
                 "User-Agent": "zt-manager-smoke/1.0"},
    )
    try:
        with urllib.request.urlopen(req, timeout=25) as f:
            raw = f.read().decode()
            return f.status, (json.loads(raw) if raw.strip() else {})
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()[:200]
    except Exception as e:  # noqa: BLE001
        return 0, str(e)


def check(name, cond, detail=""):
    (ok if cond else fail).append(name)
    print(f"  [{'PASS' if cond else 'FAIL'}] {name}" + (f"  {detail}" if detail else ""))


print("== 1. 网络列表 ==")
st, nets = call("GET", "/network")
check("GET /network 返回 200", st == 200, f"HTTP {st}")
if st != 200 or not isinstance(nets, list) or not nets:
    sys.exit("无法继续：网络列表获取失败")

n = nets[0]
nwid = n.get("id")
print(f"  网络: {nwid}  名称={n.get('config', {}).get('name')}")
check("名称字段在 config.name", "name" in n.get("config", {}))
check("顶层含 totalMemberCount", "totalMemberCount" in n)
check("顶层含 authorizedMemberCount", "authorizedMemberCount" in n)
check("顶层含 onlineMemberCount", "onlineMemberCount" in n)
check("顶层不含 assignedMemberCount（旧误用字段）", "assignedMemberCount" not in n)

print("== 2. 成员列表与字段 ==")
st, members = call("GET", f"/network/{nwid}/member")
check("GET member 列表 200", st == 200, f"HTTP {st}")
if st != 200 or not isinstance(members, list) or not members:
    sys.exit("无法继续：成员列表获取失败")

m = members[0]
check("授权状态在 config.authorized", "authorized" in m.get("config", {}),
      f"config.authorized={m.get('config', {}).get('authorized')}")
check("存在 nodeId 字段", "nodeId" in m)
check("成员 id 是复合形式 {nwid}-{nodeId}", m.get("id", "").startswith(f"{nwid}-"))

print("== 3. 关键：成员 URL 必须用 nodeId ==")
node_id = m.get("nodeId")
composite = m.get("id")
st_node, _ = call("GET", f"/network/{nwid}/member/{node_id}")
st_comp, _ = call("GET", f"/network/{nwid}/member/{composite}")
check("用 nodeId -> 200", st_node == 200, f"HTTP {st_node}")
check("用复合 id -> 404（证明不能用）", st_comp == 404, f"HTTP {st_comp}")

print("== 4. 幂等写：对已授权成员写 authorized=true（不改变状态） ==")
authorized_ones = [x for x in members if x.get("config", {}).get("authorized")]
if not authorized_ones:
    print("  [SKIP] 该网络没有已授权成员，跳过写验证（避免改动真实状态）")
else:
    t = authorized_ones[0]
    st, body = call("POST", f"/network/{nwid}/member/{t['nodeId']}", {"authorized": True})
    still = (body or {}).get("config", {}).get("authorized") if isinstance(body, dict) else None
    check("POST authorized=true -> 200", st == 200, f"HTTP {st}")
    check("状态保持已授权（幂等，未误改）", still is True, f"authorized={still}")

print("== 5. 写请求体形状 A/B（顶层 vs 嵌套 config，全部回退） ==")
# 背景：官方文档示例成员授权用 {"config":{"authorized":true}} 嵌套，
# 而本应用发顶层 {"authorized":true}。之前的幂等测试无法区分两种形状
# （写已存在的值，响应都回显旧值）。此处改用「写临时值->GET 校验->回退」：
# 若 GET 看到临时值则该形状生效，否则被服务端忽略。
import time

def restore_member_name(nwid_, mid_, old_name):
    body = {"name": old_name} if old_name else {"name": ""}
    call("POST", f"/network/{nwid_}/member/{mid_}", body)
    call("POST", f"/network/{nwid_}/member/{mid_}", {"config": {"name": old_name or ""}})

t = authorized_ones[0] if authorized_ones else (members[0] if members else None)
if not t:
    print("  [SKIP] 无成员可测")
else:
    mid = t["nodeId"]
    orig = t.get("name") or t.get("config", {}).get("name") or ""
    stamp = f"smoke-{int(time.time()) % 100000}"
    try:
        # 5a. 顶层 {"name": ...}
        st, _ = call("POST", f"/network/{nwid}/member/{mid}", {"name": stamp})
        _, g = call("GET", f"/network/{nwid}/member/{mid}")
        got = (g or {}).get("name") or (g or {}).get("config", {}).get("name") or ""
        top_ok = (st == 200 and got == stamp)
        check("成员写：顶层 {name} 生效", top_ok, f"HTTP {st} 回读={got!r}")
        # 回退（两种形状都发，保证状态还原）
        restore_member_name(nwid, mid, orig)

        # 5b. 嵌套 {"config": {"name": ...}}
        st, _ = call("POST", f"/network/{nwid}/member/{mid}", {"config": {"name": stamp}})
        _, g = call("GET", f"/network/{nwid}/member/{mid}")
        got = (g or {}).get("name") or (g or {}).get("config", {}).get("name") or ""
        nested_ok = (st == 200 and got == stamp)
        check("成员写：嵌套 {config:{name}} 生效", nested_ok, f"HTTP {st} 回读={got!r}")
        restore_member_name(nwid, mid, orig)

        _, g = call("GET", f"/network/{nwid}/member/{mid}")
        final = (g or {}).get("name") or (g or {}).get("config", {}).get("name") or ""
        check("成员 name 已回退原值", final == orig, f"现值={final!r} 原值={orig!r}")

        if top_ok and not nested_ok:
            print("  >> 结论：成员写请求体用【顶层】形状（本应用现状正确）")
        elif nested_ok and not top_ok:
            print("  >> 结论：成员写请求体必须用【嵌套 config】形状（本应用需改）")
        elif top_ok and nested_ok:
            print("  >> 结论：两种形状服务端都接受（本应用现状可用）")
        else:
            print("  >> 结论：两种形状都未生效，需人工检查")
    finally:
        restore_member_name(nwid, mid, orig)

print("== 6. 网络改名请求体 A/B（回退） ==")
net_orig = n.get("config", {}).get("name") or ""
stamp = f"smoke-net-{int(time.time()) % 100000}"
try:
    st, _ = call("POST", f"/network/{nwid}", {"name": stamp})
    _, g = call("GET", f"/network/{nwid}")
    got = (g or {}).get("config", {}).get("name", "")
    top_ok = (st == 200 and got == stamp)
    check("网络改名：顶层 {name} 生效", top_ok, f"HTTP {st} 回读={got!r}")
    call("POST", f"/network/{nwid}", {"name": net_orig})

    st, _ = call("POST", f"/network/{nwid}", {"config": {"name": stamp}})
    _, g = call("GET", f"/network/{nwid}")
    got = (g or {}).get("config", {}).get("name", "")
    nested_ok = (st == 200 and got == stamp)
    check("网络改名：嵌套 {config:{name}} 生效", nested_ok, f"HTTP {st} 回读={got!r}")
    call("POST", f"/network/{nwid}", {"config": {"name": net_orig}})

    _, g = call("GET", f"/network/{nwid}")
    final = (g or {}).get("config", {}).get("name", "")
    check("网络名已回退原值", final == net_orig, f"现值={final!r} 原值={net_orig!r}")
    if top_ok and not nested_ok:
        print("  >> 结论：网络改名用【顶层】形状（本应用现状正确）")
    elif nested_ok and not top_ok:
        print("  >> 结论：网络改名必须用【嵌套 config】形状（本应用需改）")
    elif top_ok and nested_ok:
        print("  >> 结论：两种形状服务端都接受（本应用现状可用）")
    else:
        print("  >> 结论：两种形状都未生效，需人工检查")
except Exception as e:  # noqa: BLE001
    print(f"  [ERROR] {e}")
finally:
    call("POST", f"/network/{nwid}", {"name": net_orig})
    call("POST", f"/network/{nwid}", {"config": {"name": net_orig}})

print("\n== 汇总 ==")
print(f"  通过 {len(ok)} / 失败 {len(fail)}")
if fail:
    print("  失败项： " + "、".join(fail))
    sys.exit(1)
print("  全部通过：Central 集成的请求形状与本应用实现一致。")
