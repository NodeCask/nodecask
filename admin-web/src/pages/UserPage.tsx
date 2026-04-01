import React, { useEffect, useState } from "react";
import { Badge, Box, Button, Card, Container, Dialog, Flex, Heading, Select, Table, Text, TextField, } from "@radix-ui/themes";
import Pagination from "../components/Pagination";
import { type PaginationParams, type PaginationResponse, pull, push, SysError } from "../api";
import UserAddDialog, { type UserAddForm } from "../components/UserAddDialog";

export interface User {
    id: number;
    username: string;
    credit_score: number;
    coins: number;
    topics: number;
    replies: number;
    email: string;
    role: string;
    active: boolean;
    bio: string;
    address: string;
    timezone: string;
    language: string;
    public_email: boolean;
    created_at: string;
    updated_at: string;
    last_access_date: string;
}

export interface UserSearchParams extends PaginationParams {
    username?: string;
    active?: string; // "true" | "false"
    role?: string; // "admin" | "moderator" | "user"
    per_page?: number;
}

// 获取用户列表
async function getUsers(
    params: UserSearchParams = {}
): Promise<PaginationResponse<User>> {
    const queryParams: Record<string, string> = {};

    if (params.p) queryParams.p = String(params.p);
    if (params.username) queryParams.username = params.username;
    if (params.active) queryParams.active = params.active;
    if (params.role) queryParams.role = params.role;
    if (params.per_page) queryParams.per_page = String(params.per_page);

    return pull<PaginationResponse<User>>(`/users?${new URLSearchParams(queryParams).toString()}`);
}

// 更新用户状态
async function updateUserStatus(
    id: number,
    action: "enable" | "disable"
): Promise<void> {
    return push(`/users/${id}/status`, { action });
}

// 更新用户角色
async function updateUserRole(
    id: number,
    role: string
): Promise<void> {
    return push(`/users/${id}/role`, { role });
}

// 重置用户密码
async function resetUserPassword(
    id: number,
    password: string
): Promise<void> {
    return push(`/users/${id}/reset-password`, { password });
}

// 添加用户
async function addUser(data: UserAddForm): Promise<void> {
    return push(`/users`, data);
}

// --- Components ---

interface UserFiltersProps {
    searchQuery: string;
    onSearchChange: (value: string) => void;
    statusFilter: string;
    onStatusFilterChange: (value: string) => void;
    roleFilter: string;
    onRoleFilterChange: (value: string) => void;
    onReset: () => void;
    onAddUser: () => void;
}

const UserFilters: React.FC<UserFiltersProps> = ({
    searchQuery,
    onSearchChange,
    statusFilter,
    onStatusFilterChange,
    roleFilter,
    onRoleFilterChange,
    onReset,
    onAddUser,
}) => {
    return (
        <Card>
            <Flex gap="3" wrap="wrap" align="end">
                <Box style={{ flex: "1", minWidth: "200px" }}>
                    <Text as="label" size="2" weight="medium" mb="1">
                        搜索
                    </Text>
                    <TextField.Root
                        placeholder="搜索用户名..."
                        value={searchQuery}
                        onChange={(e) => onSearchChange(e.target.value)}
                        size="2"
                    />
                </Box>

                <Box style={{ minWidth: "150px" }}>
                    <Text as="label" size="2" weight="medium" mb="1">
                        状态
                    </Text>
                    <Select.Root value={statusFilter} onValueChange={onStatusFilterChange}>
                        <Select.Trigger style={{ width: "100%" }} />
                        <Select.Content>
                            <Select.Item value="all">全部状态</Select.Item>
                            <Select.Item value="active">正常</Select.Item>
                            <Select.Item value="inactive">禁用</Select.Item>
                        </Select.Content>
                    </Select.Root>
                </Box>

                <Box style={{ minWidth: "150px" }}>
                    <Text as="label" size="2" weight="medium" mb="1">
                        角色
                    </Text>
                    <Select.Root value={roleFilter} onValueChange={onRoleFilterChange}>
                        <Select.Trigger style={{ width: "100%" }} />
                        <Select.Content>
                            <Select.Item value="all">全部角色</Select.Item>
                            <Select.Item value="administrator">超级管理员</Select.Item>
                            <Select.Item value="moderator">管理员</Select.Item>
                            <Select.Item value="user">普通用户</Select.Item>
                        </Select.Content>
                    </Select.Root>
                </Box>

                <Button variant="soft" onClick={onReset}>
                    重置筛选
                </Button>
                <Button onClick={onAddUser} color="green">
                    添加用户
                </Button>
            </Flex>
        </Card>
    );
};

interface UserTableProps {
    users: User[];
    loading: boolean;
    onRoleChange: (user: User) => void;
    onStatusToggle: (user: User) => void;
    onResetPassword: (user: User) => void;
}

const UserTable: React.FC<UserTableProps> = ({
    users,
    loading,
    onRoleChange,
    onStatusToggle,
    onResetPassword,
}) => {
    const getStatusBadge = (active: boolean) => {
        return active ? (
            <Badge color="green">正常</Badge>
        ) : (
            <Badge color="red">禁用</Badge>
        );
    };

    const getRoleBadge = (role: string) => {
        const colors: Record<string, "blue" | "purple" | "gray"> = {
            administrator: "blue",
            admin: "blue", // 兼容旧数据
            moderator: "purple",
            user: "gray",
        };
        const labels: Record<string, string> = {
            administrator: "超级管理员",
            admin: "超级管理员", // 兼容旧数据
            moderator: "管理员",
            user: "普通用户",
        };
        return <Badge color={colors[role] || "gray"}>{labels[role] || role}</Badge>;
    };

    return (
        <Card variant="ghost">
            <Table.Root variant="surface">
                <Table.Header>
                    <Table.Row>
                        <Table.ColumnHeaderCell>ID</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "120px" }}>用户名</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "180px" }}>邮箱</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>信誉分</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>金币</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>发帖数量</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>评论数量</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell>状态</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>角色</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "120px" }}>创建时间</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "120px" }}>最后访问</Table.ColumnHeaderCell>
                        <Table.ColumnHeaderCell style={{ minWidth: "180px" }}>操作</Table.ColumnHeaderCell>
                    </Table.Row>
                </Table.Header>

                <Table.Body>
                    {users.map((user) => (
                        <Table.Row key={user.id}>
                            <Table.Cell>{user.id}</Table.Cell>
                            <Table.Cell>
                                <Text weight="medium">{user.username}</Text>
                            </Table.Cell>
                            <Table.Cell>{user.email || "-"}</Table.Cell>
                            <Table.Cell>{user.credit_score}</Table.Cell>
                            <Table.Cell>{user.coins}</Table.Cell>
                            <Table.Cell>{user.topics}</Table.Cell>
                            <Table.Cell>{user.replies}</Table.Cell>
                            <Table.Cell>{getStatusBadge(user.active)}</Table.Cell>
                            <Table.Cell>{getRoleBadge(user.role)}</Table.Cell>
                            <Table.Cell>
                                {new Date(user.created_at).toLocaleDateString("zh-CN")}
                            </Table.Cell>
                            <Table.Cell>
                                {new Date(user.last_access_date).toLocaleDateString("zh-CN")}
                            </Table.Cell>
                            <Table.Cell>
                                <Flex gap="2">
                                    <Button
                                        size="1"
                                        variant="soft"
                                        color="crimson"
                                        onClick={() => onResetPassword(user)}
                                    >
                                        重置密码
                                    </Button>
                                    <Button
                                        size="1"
                                        variant="soft"
                                        onClick={() => onRoleChange(user)}
                                    >
                                        更改角色
                                    </Button>
                                    <Button
                                        size="1"
                                        variant="soft"
                                        color={user.active ? "orange" : "green"}
                                        onClick={() => onStatusToggle(user)}
                                        disabled={loading}
                                    >
                                        {user.active ? "禁用" : "启用"}
                                    </Button>
                                </Flex>
                            </Table.Cell>
                        </Table.Row>
                    ))}
                </Table.Body>
            </Table.Root>

            {users.length === 0 && !loading && (
                <Card>
                    <Box p="4" style={{ textAlign: "center" }}>
                        <Text color="gray">暂无数据</Text>
                    </Box>
                </Card>
            )}
        </Card>
    );
};

interface RoleDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    user: User | null;
    currentRole: string;
    onRoleChange: (value: string) => void;
    onConfirm: () => Promise<void>;
}

const RoleDialog: React.FC<RoleDialogProps> = ({
    open,
    onOpenChange,
    user,
    currentRole,
    onRoleChange,
    onConfirm,
}) => {
    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Content style={{ maxWidth: 450 }}>
                <Dialog.Title>更改角色</Dialog.Title>
                <Dialog.Description size="2" mb="4">
                    请选择用户 <strong>{user?.username}</strong> 的新角色。
                </Dialog.Description>

                <Flex direction="column" gap="3">
                    <Box>
                        <Text as="label" size="2" weight="medium" mb="1">
                            角色
                        </Text>
                        <Select.Root value={currentRole} onValueChange={onRoleChange}>
                            <Select.Trigger style={{ width: "100%" }} />
                            <Select.Content>
                                <Select.Item value="administrator">超级管理员</Select.Item>
                                <Select.Item value="moderator">管理员</Select.Item>
                                <Select.Item value="user">普通用户</Select.Item>
                            </Select.Content>
                        </Select.Root>
                    </Box>
                </Flex>

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray">
                            取消
                        </Button>
                    </Dialog.Close>
                    <Button onClick={onConfirm}>
                        确定
                    </Button>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};

interface StatusDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    user: User | null;
    onConfirm: () => Promise<void>;
}

const StatusDialog: React.FC<StatusDialogProps> = ({
    open,
    onOpenChange,
    user,
    onConfirm,
}) => {
    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Content style={{ maxWidth: 450 }}>
                <Dialog.Title>{user?.active ? "禁用用户" : "启用用户"}</Dialog.Title>
                <Dialog.Description size="2" mb="4">
                    确定要{user?.active ? "禁用" : "启用"}用户 <strong>{user?.username}</strong> 吗？
                </Dialog.Description>

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray">
                            取消
                        </Button>
                    </Dialog.Close>
                    <Button
                        color={user?.active ? "orange" : "green"}
                        onClick={onConfirm}
                    >
                        确定
                    </Button>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};

interface PasswordDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    user: User | null;
    password: string;
    onPasswordChange: (value: string) => void;
    onConfirm: () => Promise<void>;
}

const PasswordDialog: React.FC<PasswordDialogProps> = ({
    open,
    onOpenChange,
    user,
    password,
    onPasswordChange,
    onConfirm,
}) => {
    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Content style={{ maxWidth: 450 }}>
                <Dialog.Title>重置密码</Dialog.Title>
                <Dialog.Description size="2" mb="4">
                    正在为用户 <strong>{user?.username}</strong> 重置密码。
                </Dialog.Description>

                <Flex direction="column" gap="3">
                    <Box>
                        <Text as="label" size="2" weight="medium" mb="1">
                            新密码
                        </Text>
                        <TextField.Root
                            value={password}
                            onChange={(e) => onPasswordChange(e.target.value)}
                        />
                        <Text size="1" color="gray" mt="1">
                            已为您自动生成随机密码，您也可以手动修改。
                        </Text>
                    </Box>
                </Flex>

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray">
                            取消
                        </Button>
                    </Dialog.Close>
                    <Button
                        color="crimson"
                        onClick={onConfirm}
                    >
                        确定重置
                    </Button>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};

const UserPage: React.FC = () => {
    const [currentPage, setCurrentPage] = useState(1);
    const [itemsPerPage, setItemsPerPage] = useState(20);
    const [searchQuery, setSearchQuery] = useState("");
    const [statusFilter, setStatusFilter] = useState("all");
    const [roleFilter, setRoleFilter] = useState("all");
    const [users, setUsers] = useState<User[]>([]);
    const [total, setTotal] = useState(0);
    const [totalPages, setTotalPages] = useState(0);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Dialog states
    const [roleDialogOpen, setRoleDialogOpen] = useState(false);
    const [statusDialogOpen, setStatusDialogOpen] = useState(false);
    const [resetPasswordDialogOpen, setResetPasswordDialogOpen] = useState(false);
    const [addUserDialogOpen, setAddUserDialogOpen] = useState(false);
    const [selectedUser, setSelectedUser] = useState<User | null>(null);
    const [tempRole, setTempRole] = useState<string>("");
    const [newPassword, setNewPassword] = useState<string>("");

    // 加载用户列表
    const loadUsers = async () => {
        setLoading(true);
        setError(null);
        try {
            const params: UserSearchParams = {
                p: currentPage,
                per_page: itemsPerPage,
            };

            if (searchQuery) params.username = searchQuery;
            if (statusFilter !== "all") {
                params.active = statusFilter === "active" ? "true" : "false";
            }
            if (roleFilter !== "all") params.role = roleFilter;

            const response = await getUsers(params);
            setUsers(response.data);
            setTotal(response.total);
            setTotalPages(response.total_page);
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "加载用户列表失败";
            setError(errorMessage);
            console.error("Failed to load users:", err);
        } finally {
            setLoading(false);
        }
    };

    // 页面加载和筛选条件变化时重新加载
    useEffect(() => {
        loadUsers();
    }, [currentPage, searchQuery, statusFilter, roleFilter, itemsPerPage]);

    // 重置筛选条件时回到第一页
    useEffect(() => {
        setCurrentPage(1);
    }, [searchQuery, statusFilter, roleFilter, itemsPerPage]);

    const handleToggleStatusConfirm = async () => {
        if (!selectedUser) return;
        try {
            const action = selectedUser.active ? "disable" : "enable";
            await updateUserStatus(selectedUser.id, action);
            setStatusDialogOpen(false);
            await loadUsers();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "更新用户状态失败";
            alert(errorMessage);
            console.error("Failed to update user status:", err);
        }
    };

    const handleRoleChangeConfirm = async () => {
        if (!selectedUser) return;
        try {
            await updateUserRole(selectedUser.id, tempRole);
            setRoleDialogOpen(false);
            await loadUsers();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "更新用户角色失败";
            alert(errorMessage);
            console.error("Failed to update user role:", err);
        }
    };

    const handleResetPasswordConfirm = async () => {
        if (!selectedUser || !newPassword) return;
        try {
            await resetUserPassword(selectedUser.id, newPassword);
            setResetPasswordDialogOpen(false);
            alert(`用户 ${selectedUser.username} 的密码已重置成功。`);
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "重置密码失败";
            alert(errorMessage);
            console.error("Failed to reset password:", err);
        }
    };

    const handleAddUserConfirm = async (formData: UserAddForm) => {
        try {
            await addUser(formData);
            setAddUserDialogOpen(false);
            await loadUsers();
            alert(`用户 ${formData.username} 创建成功`);
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "添加用户失败";
            alert(errorMessage);
            console.error("Failed to add user:", err);
            throw new Error(errorMessage); // ensure loading state terminates
        }
    };

    return (
        <Container size="4">
            <Flex direction="column" gap="5">
                {/* Header */}
                <Box>
                    <Heading as="h1" size="6" mb="2">
                        用户管理
                    </Heading>
                    <Text as="p" size="3" color="gray">
                        管理论坛用户账号和权限
                    </Text>
                </Box>

                <UserFilters
                    searchQuery={searchQuery}
                    onSearchChange={setSearchQuery}
                    statusFilter={statusFilter}
                    onStatusFilterChange={setStatusFilter}
                    roleFilter={roleFilter}
                    onRoleFilterChange={setRoleFilter}
                    onReset={() => {
                        setSearchQuery("");
                        setStatusFilter("all");
                        setRoleFilter("all");
                    }}
                    onAddUser={() => setAddUserDialogOpen(true)}
                />

                {/* Error Message */}
                {error && (
                    <Card>
                        <Text color="red" size="2">
                            错误: {error}
                        </Text>
                    </Card>
                )}

                {/* Results Summary */}
                <Text size="2" color="gray">
                    找到 {total} 个用户{loading && " (加载中...)"}
                </Text>

                <UserTable
                    users={users}
                    loading={loading}
                    onRoleChange={(user) => {
                        setSelectedUser(user);
                        setTempRole(user.role);
                        setRoleDialogOpen(true);
                    }}
                    onStatusToggle={(user) => {
                        setSelectedUser(user);
                        setStatusDialogOpen(true);
                    }}
                    onResetPassword={(user) => {
                        setSelectedUser(user);
                        setNewPassword(Math.random().toString(36).slice(-8) + Math.random().toString(36).slice(-8));
                        setResetPasswordDialogOpen(true);
                    }}
                />

                <Card>
                    <Pagination
                        currentPage={currentPage}
                        totalPages={totalPages}
                        onPageChange={setCurrentPage}
                        itemsPerPage={itemsPerPage}
                        onItemsPerPageChange={setItemsPerPage}
                    />
                </Card>
            </Flex>

            <RoleDialog
                open={roleDialogOpen}
                onOpenChange={setRoleDialogOpen}
                user={selectedUser}
                currentRole={tempRole}
                onRoleChange={setTempRole}
                onConfirm={handleRoleChangeConfirm}
            />

            <StatusDialog
                open={statusDialogOpen}
                onOpenChange={setStatusDialogOpen}
                user={selectedUser}
                onConfirm={handleToggleStatusConfirm}
            />

            <PasswordDialog
                open={resetPasswordDialogOpen}
                onOpenChange={setResetPasswordDialogOpen}
                user={selectedUser}
                password={newPassword}
                onPasswordChange={setNewPassword}
                onConfirm={handleResetPasswordConfirm}
            />

            <UserAddDialog
                open={addUserDialogOpen}
                onOpenChange={setAddUserDialogOpen}
                onConfirm={handleAddUserConfirm}
            />
        </Container>
    );
};

export default UserPage;
