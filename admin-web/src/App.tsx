import "./App.css";
import { HashRouter, Routes, Route, Navigate, useNavigate } from "react-router";
import { useEffect } from "react";
import { setupAuthInterceptor } from "./api";
import { Theme } from "@radix-ui/themes";
import "@radix-ui/themes/styles.css";
import { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor";
loader.config({ monaco });

// 布局组件
import MainLayout from "./layouts/MainLayout";

// 页面组件
import HomePage from "./pages/HomePage";
import LoginPage from "./pages/LoginPage";
import UserPage from "./pages/UserPage.tsx";
import TopicPage from "./pages/TopicPage.tsx";
import NodePage from "./pages/NodePage.tsx";
import SettingsPage from "./pages/SettingsPage";
import LinksPage from "./pages/LinksPage";
import CustomPage from "./pages/CustomPage";
import EmailPage from "./pages/EmailPage.tsx";
import InviteCodePage from "./pages/InviteCodePage.tsx";
import TokenPage from "./pages/TokenPage";
import FileManagerPage from "./pages/FileManagerPage";

function AuthInterceptor() {
    const navigate = useNavigate();
    useEffect(() => {
        setupAuthInterceptor(navigate);
    }, [navigate]);
    return null;
}

function App() {
    return (
        <HashRouter>
            <AuthInterceptor />
            <Theme>
                <Routes>
                    {/* 登录页面 - 不需要认证 */}
                    <Route path="/login" element={<LoginPage />} />

                    {/* 主布局 - 需要认证 */}
                    <Route path="/" element={<MainLayout />}>
                        {/* 首页 */}
                        <Route index element={<HomePage />} />

                        {/* 用户管理页面 */}
                        <Route path="users" element={<UserPage />} />

                        {/* 帖子管理页面 */}
                        <Route path="posts" element={<TopicPage />} />

                        {/* 分类管理页面 */}
                        <Route path="categories" element={<NodePage />} />

                        {/* 自定义页面管理 */}
                        <Route path="pages" element={<CustomPage />} />

                        {/* 系统设置页面 */}
                        <Route path="settings" element={<SettingsPage />} />

                        {/* 邮件队列页面 */}
                        <Route path="emails" element={<EmailPage />} />

                        {/* 邀请码管理页面 */}
                        <Route path="invites" element={<InviteCodePage />} />

                        {/* 令牌管理页面 */}
                        <Route path="tokens" element={<TokenPage />} />

                        {/* 页面导航链接管理 */}
                        <Route path="links" element={<LinksPage />} />

                        {/* 文件管理器 */}
                        <Route path="files" element={<FileManagerPage />} />

                        {/* 个人资料页面 - 占位符 */}
                        <Route
                            path="profile"
                            element={
                                <div style={{ padding: "20px" }}>
                                    <h2>个人资料页面</h2>
                                    <p>个人资料功能开发中...</p>
                                </div>
                            }
                        />

                        {/* 404 页面 */}
                        <Route path="*" element={<Navigate to="/" replace />} />
                    </Route>
                </Routes>
            </Theme>
        </HashRouter>
    );
}

export default App;
