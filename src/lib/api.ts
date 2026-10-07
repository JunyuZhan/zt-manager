import { invoke } from "@tauri-apps/api/core";

export type CentralMode = "auto" | "legacy" | "new";

export interface Settings {
  token: string;
  centralMode: CentralMode;
  orgId: string | null;
  legacyBase?: string | null;
  newBase?: string | null;
}

export interface ServiceState {
  state: string;
  pid: number | null;
}

export interface PingResult {
  host: string;
  latencyMs: number;
}

export interface AuthStatus {
  loggedIn: boolean;
  email?: string;
  mode?: "central" | "keycloak";
  expiresAt?: number | null;
  /** 已配置 token 粘贴模式（未登录会话时的备用认证） */
  tokenConfigured?: boolean;
}

export type MemberAction = "authorize" | "deauthorize" | "reject";

export const api = {
  settingsLoad: () => invoke<Settings>("settings_load"),
  settingsSave: (settings: Settings) => invoke<void>("settings_save", { settings }),
  centralDetect: () => invoke<string>("central_detect"),

  authLogin: (email: string, password: string, otp?: string) =>
    invoke<any>("auth_login", { email, password, otp: otp ?? null }),
  authLogout: () => invoke<void>("auth_logout"),
  authStatus: () => invoke<AuthStatus>("auth_status"),

  serviceQuery: () => invoke<ServiceState>("service_query"),
  serviceControl: (action: "start" | "stop" | "restart") =>
    invoke<ServiceState>("service_control", { action }),

  agentStatus: () => invoke<any>("agent_status"),
  agentNetworks: () => invoke<string[]>("agent_networks"),
  agentNetworkDetail: (nwid: string) =>
    invoke<any>("agent_network_detail", { nwid }),
  agentJoin: (nwid: string) => invoke<any>("agent_join", { nwid }),
  agentLeave: (nwid: string) => invoke<any>("agent_leave", { nwid }),
  agentPeers: () => invoke<any>("agent_peers"),

  centralNetworks: () => invoke<any>("central_networks"),
  centralNetwork: (nwid: string) => invoke<any>("central_network", { nwid }),
  centralCreateNetwork: (name: string) =>
    invoke<any>("central_create_network", { name }),
  centralDeleteNetwork: (nwid: string) =>
    invoke<any>("central_delete_network", { nwid }),
  centralUpdateNetwork: (nwid: string, body: any) =>
    invoke<any>("central_update_network", { nwid, body }),
  centralMembers: (nwid: string) => invoke<any>("central_members", { nwid }),
  centralUpdateMember: (nwid: string, mid: string, body: any) =>
    invoke<any>("central_update_member", { nwid, mid, body }),
  centralMemberAction: (nwid: string, mid: string, action: MemberAction) =>
    invoke<any>("central_member_action", { nwid, mid, action }),
  centralDeleteMember: (nwid: string, mid: string) =>
    invoke<any>("central_delete_member", { nwid, mid }),
  centralOrgs: () => invoke<any>("central_orgs"),

  controllerNetworks: () => invoke<any>("controller_networks"),
  controllerNetwork: (nwid: string) => invoke<any>("controller_network", { nwid }),
  controllerCreate: () => invoke<any>("controller_create"),
  controllerUpdate: (nwid: string, body: any) =>
    invoke<any>("controller_update", { nwid, body }),
  controllerMembers: (nwid: string) => invoke<any>("controller_members", { nwid }),
  controllerUpdateMember: (nwid: string, mid: string, body: any) =>
    invoke<any>("controller_update_member", { nwid, mid, body }),

  pingHost: (host: string) => invoke<PingResult>("ping_host", { host }),
};
