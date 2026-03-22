// 分页参数
export interface PaginationParams {
    p?: number; // 页码
}

// 分页响应
export interface PaginationResponse<T> {
    data: T[];
    total: number;
    per_page: number;
    total_page: number;
    current_page: number;
}

import type { NavigateFunction } from "react-router";

let globalNavigate: NavigateFunction | null = null;

export function setupAuthInterceptor(navigate: NavigateFunction) {
    globalNavigate = navigate;
}

const token = {
    token: localStorage.getItem("admin_token")
};

export function setToken(t: string) {
    localStorage.setItem("admin_token", t);
    token.token = t;
}

export function hasToken(): boolean {
    return !!token.token
}

function errWrap<T>(err: T) {
    if (err instanceof SysError) throw err;
    if (err instanceof SyntaxError) throw new SysError(-1, "响应数据格式错误");
    throw new SysError(-1, err instanceof Error ? err.message : "网络请求失败");
}

/**
 * 发送一个 GET 请求
 * @param path 查询路径包括查询参数
 */
export async function pull<T>(path: string): Promise<T> {
    const new_path = path.startsWith("/") ? path : `/${path}`;
    return fetch(`/mod${new_path}`, {
        method: "GET",
        headers: [
            ["credentials", "same-origin"],
            ["token", token.token || ""],
            ["Accept", "application/json"],
        ]
    })
        .then(resp => resp.json())
        .then((data: { code: number, message?: string, data?: T }) => {
            if (data && data.code === 0) {
                return data.data as T
            }
            if (data && data.code === 99) { // 未登录
                localStorage.removeItem("admin_token");
                token.token = null;
                if (globalNavigate) {
                    const currentPath = window.location.hash.slice(1) || "/";
                    if (!currentPath.startsWith("/login")) {
                        globalNavigate("/login", { state: { from: currentPath } });
                    }
                } else {
                    window.location.hash = "/login";
                }
                return Promise.reject(new SysError(data.code, "未登录"));
            }
            return Promise.reject(new SysError(data.code, data.message || `unknown error, code: ${data.code}`, data.data));
        }).catch(errWrap) as Promise<T>;
}

/**
 * 发送一个 POST 请求
 * @param path 查询路径包括查询参数
 * @param data 附加数据，自动编码成为 JSON 格式
 */
export async function push<T, D>(path: string, data?: D): Promise<T> {
    const new_path = path.startsWith("/") ? path : `/${path}`;
    const headers: [string, string][] = [
        ["Accept", "application/json"],
        ["credentials", "same-origin"],
        ["token", token.token || ""],
    ];
    if (data) {
        headers.push(["Content-Type", "application/json"])
    }
    return fetch(`/mod${new_path}`, {
        headers: headers,
        method: "POST",
        body: data ? JSON.stringify(data) : undefined,
    })
        .then(resp => resp.json())
        .then((data: { code: number, message?: string, data?: T }) => {
            if (data && data.code === 0) {
                return data.data as T
            }
            if (data && data.code === 99) { // 未登录
                localStorage.removeItem("admin_token");
                token.token = null;
                if (globalNavigate) {
                    const currentPath = window.location.hash.slice(1) || "/";
                    if (!currentPath.startsWith("/login")) {
                        globalNavigate("/login", { state: { from: currentPath } });
                    }
                } else {
                    window.location.hash = "/login";
                }
                return Promise.reject(new SysError(data.code, "未登录"));
            }
            return Promise.reject(new SysError(data.code, data.message || `unknown error, code: ${data.code}`, data.data));
        }).catch(errWrap) as Promise<T>;

}

export interface AccessTokenResponse {
    token: string;
    totp: boolean;
    message: string;
}

export async function login(username: string, password: string): Promise<AccessTokenResponse> {
    return fetch(`/get-access-token`, {
        headers: [
            ["Content-Type", "application/json"],
            ["credentials", "same-origin"],
            ["token", token.token || ""]
        ],
        method: "POST",
        body: JSON.stringify({ username, password })
    })
        .then(resp => resp.json())
        .then((data: { code: number, message?: string, data?: AccessTokenResponse }) => {
            if (data && data.code === 0 && data.data) {
                setToken(data.data.token);
                return data.data;
            }
            return Promise.reject(new SysError(data.code, data.message || `unknown error, code: ${data.code}`, data.data));
        }).catch(errWrap) as Promise<AccessTokenResponse>;
}

export async function verifyTotp(username: string, code: string): Promise<AccessTokenResponse> {
    return fetch(`/get-access-token/totp-challenge`, {
        headers: [
            ["Content-Type", "application/json"],
            ["credentials", "same-origin"]
        ],
        method: "POST",
        body: JSON.stringify({ username, code })
    })
        .then(resp => resp.json())
        .then((data: { code: number, message?: string, data?: AccessTokenResponse }) => {
            if (data && data.code === 0 && data.data) {
                setToken(data.data.token);
                return data.data;
            }
            return Promise.reject(new SysError(data.code, data.message || `unknown error, code: ${data.code}`, data.data));
        }).catch(errWrap) as Promise<AccessTokenResponse>;
}

export async function logout() {
    try {
        await push("/logout");
    } catch (e) {
        console.error(e);
    }
    localStorage.removeItem("admin_token");
    token.token = "";
}

export class SysError<T> extends Error {
    code: number;
    data?: T;

    constructor(code: number, message: string, data?: T) {
        super(message);
        this.name = "ApiError";
        this.code = code;
        this.data = data;
    }
}

export interface UserInfo {
    uid: number;
    username: string;
    role?: string;
    [key: string]: unknown;
}

export async function me() {
    return pull<UserInfo>("/me");
}
