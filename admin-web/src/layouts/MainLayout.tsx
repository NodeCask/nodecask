import React from "react";
import { Outlet, useNavigate, Link, useLocation } from "react-router";
import {
  Box,
  Button,
  Container,
  DropdownMenu,
  Flex,
  Heading,
  Link as RadixLink,
  Separator,
  Text,
} from "@radix-ui/themes";
import { useAuth } from "../hooks";

import { HamburgerMenuIcon } from "@radix-ui/react-icons";

const MainLayout: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { isAuthenticated, userInfo, logout, isLoading } = useAuth();

  const handleLogout = async () => {
    await logout();
  };

  // 导航菜单项
  const navItems = [
    { path: "", label: "首页" },
    { path: "users", label: "用户管理" },
    { path: "posts", label: "帖子管理" },
    { path: "categories", label: "节点管理" },
    { path: "invites", label: "邀请码管理" },
    { path: "tokens", label: "机器人令牌" },
    { path: "pages", label: "自定义页面" },
    { path: "emails", label: "邮件队列" },
      { path: "links", label: "导航链接" },
    { path: "files", label: "文件管理" },
    { path: "settings", label: "系统设置" },
  ];

  return (
    <Flex direction="column" style={{ minHeight: "100vh" }}>
      {/* 顶部导航栏 */}
      <Box
        asChild
        style={{
          backgroundColor: "var(--gray-1)",
          borderBottom: "1px solid var(--gray-4)",
          padding: "12px 0",
        }}
      >
        <header>
          <Container size="4">
            <Flex align="center" justify="between">
              {/* Logo 和标题 */}
              <Flex align="center" gap="4">
                <Link
                  to="/"
                  style={{ textDecoration: "none", color: "inherit" }}
                >
                  <Flex align="center" gap="2">
                    <Box
                      style={{
                        width: "32px",
                        height: "32px",
                        backgroundColor: "var(--accent-9)",
                        borderRadius: "6px",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                      }}
                    >
                      <Text size="2" weight="bold" style={{ color: "white" }}>
                        A
                      </Text>
                    </Box>
                    <Heading as="h1" size="4">
                      论坛管理系统
                    </Heading>
                  </Flex>
                </Link>
              </Flex>

              {/* 桌面端导航菜单 (md 以上显示) */}
              <Flex align="center" gap="4" display={{ initial: "none", md: "flex" }}>
                <Flex asChild gap="4">
                  <nav>
                    {navItems.map((item) => {
                      const currentPath = location.pathname;
                      const itemPath = item.path === "" ? "/" : `/${item.path}`;
                      const isActive = currentPath === itemPath;

                      return (
                        <Link
                          key={item.path}
                          to={item.path}
                          style={{
                            textDecoration: "none",
                            color: isActive
                              ? "var(--accent-11)"
                              : "var(--gray-11)",
                            fontWeight: isActive ? "600" : "400",
                          }}
                        >
                          <Text size="2">{item.label}</Text>
                        </Link>
                      );
                    })}
                  </nav>
                </Flex>

                <Separator orientation="vertical" style={{ height: "24px" }} />

                {/* 用户菜单 */}
                {isAuthenticated ? (
                  <DropdownMenu.Root>
                    <DropdownMenu.Trigger>
                      <Button variant="ghost" size="2" disabled={isLoading}>
                        <Flex align="center" gap="2">
                          <Box
                            style={{
                              width: "24px",
                              height: "24px",
                              backgroundColor: "var(--accent-9)",
                              borderRadius: "50%",
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                            }}
                          >
                            <Text
                              size="1"
                              weight="bold"
                              style={{ color: "white" }}
                            >
                              {userInfo?.username?.charAt(0) || "A"}
                            </Text>
                          </Box>
                          <Text size="2">
                            {userInfo?.username ||
                              (isLoading ? "加载中..." : "管理员")}
                          </Text>
                        </Flex>
                      </Button>
                    </DropdownMenu.Trigger>
                    <DropdownMenu.Content>
                      <DropdownMenu.Item onClick={() => navigate("/profile")}>
                        个人资料
                      </DropdownMenu.Item>
                      <DropdownMenu.Separator />
                      <DropdownMenu.Item color="red" onClick={handleLogout}>
                        退出登录
                      </DropdownMenu.Item>
                    </DropdownMenu.Content>
                  </DropdownMenu.Root>
                ) : (
                  <Button size="2" onClick={() => navigate("/login")}>
                    登录
                  </Button>
                )}
              </Flex>

              {/* 移动端菜单按钮 (md 以下显示) */}
              <Flex align="center" gap="3" display={{ initial: "flex", md: "none" }}>
                {/* 移动端也显示简单的用户头像或登录按钮 */}
                 {isAuthenticated ? (
                   <DropdownMenu.Root>
                     <DropdownMenu.Trigger>
                        <Box
                           style={{
                             width: "32px",
                             height: "32px",
                             backgroundColor: "var(--accent-9)",
                             borderRadius: "50%",
                             display: "flex",
                             alignItems: "center",
                             justifyContent: "center",
                             cursor: "pointer",
                           }}
                         >
                           <Text
                             size="2"
                             weight="bold"
                             style={{ color: "white" }}
                           >
                             {userInfo?.username?.charAt(0) || "A"}
                           </Text>
                         </Box>
                     </DropdownMenu.Trigger>
                     <DropdownMenu.Content>
                        <DropdownMenu.Item onClick={() => navigate("/profile")}>
                          个人资料
                        </DropdownMenu.Item>
                        <DropdownMenu.Separator />
                        <DropdownMenu.Item color="red" onClick={handleLogout}>
                          退出登录
                        </DropdownMenu.Item>
                     </DropdownMenu.Content>
                   </DropdownMenu.Root>
                 ) : (
                    <Button size="2" onClick={() => navigate("/login")}>
                      登录
                    </Button>
                 )}

                <Flex style={{width: "60px",height: "40px"}} align="center" justify="center">

                <DropdownMenu.Root>
                  <DropdownMenu.Trigger>
                    <Button variant="ghost" size="3">
                      <HamburgerMenuIcon width="20" height="20" />
                    </Button>
                  </DropdownMenu.Trigger>
                  <DropdownMenu.Content align="end">
                    {navItems.map((item) => {
                      const currentPath = location.pathname;
                      const itemPath = item.path === "" ? "/" : `/${item.path}`;
                      const isActive = currentPath === itemPath;
                      return (
                        <DropdownMenu.Item
                           key={item.path}
                           onClick={() => navigate(itemPath)}
                           style={{
                             backgroundColor: isActive ? "var(--accent-3)" : undefined,
                             color: isActive ? "var(--accent-11)" : undefined,
                           }}
                        >
                          <Text weight={isActive ? "bold" : "regular"}>{item.label}</Text>
                        </DropdownMenu.Item>
                      )
                    })}
                  </DropdownMenu.Content>
                </DropdownMenu.Root>
                </Flex>
              </Flex>
            </Flex>
          </Container>
        </header>
      </Box>

      {/* 主内容区域 */}
      <Box asChild style={{ flex: 1, padding: "24px 0" }}>
        <main>
          <Container size="4">
            <Outlet />
          </Container>
        </main>
      </Box>

      {/* 页脚 */}
      <Box
        asChild
        style={{
          backgroundColor: "var(--gray-1)",
          borderTop: "1px solid var(--gray-4)",
          padding: "20px 0",
        }}
      >
        <footer>
          <Container size="4">
            <Flex direction="column" gap="2" align="center">
              <Text size="1" color="gray">
                Powered by{" "}
                <RadixLink
                  href="https://github.com/NodeCask"
                  target="_blank"
                  rel="noopener noreferrer"
                  color="gray"
                  highContrast
                >
                  NodeCask
                </RadixLink>
              </Text>
            </Flex>
          </Container>
        </footer>
      </Box>
    </Flex>
  );
};

export default MainLayout;
