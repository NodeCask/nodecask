import React, { useEffect, useState } from "react";
import {
    Box,
    Button,
    Card,
    Checkbox,
    Container,
    Dialog,
    Flex,
    Heading,
    Table,
    Tabs,
    Text,
    TextArea,
    TextField,
} from "@radix-ui/themes";
import { pull, push, SysError } from "../api";

// ============= Types =============

export interface Category {
    id: number;
    name: string;
    slug: string;
    description: string;
    show_in_list: boolean;
    created_at: string;
    member_access_required: boolean;
    moderator_access_required: boolean;
    isolated: boolean;
    access_only: boolean;
    topic_reward: number;
    comment_reward: number;
}

interface CategoryCreateParams {
    name: string;
    slug: string;
    description: string;
    show_in_list: boolean;
    member_access_required: boolean;
    moderator_access_required: boolean;
    isolated: boolean;
    access_only: boolean;
    topic_reward: number;
    comment_reward: number;
}

interface CategoryUpdateParams {
    name: string;
    slug: string;
    description: string;
    show_in_list: boolean;
    member_access_required: boolean;
    moderator_access_required: boolean;
    isolated: boolean;
    access_only: boolean;
    topic_reward: number;
    comment_reward: number;
}

// ============= API Functions =============

async function getCategories(): Promise<Category[]> {
    return pull<Category[]>("/nodes");
}

async function createCategory(params: CategoryCreateParams): Promise<void> {
    return push("/nodes", params);
}

async function updateCategory(id: number, params: CategoryUpdateParams): Promise<void> {
    return push(`/nodes/${id}`, params);
}

async function deleteCategory(id: number): Promise<void> {
    return push(`/nodes/${id}/delete`);
}

async function getNodeAttributes(id: number): Promise<[string, string][]> {
    return pull<[string, string][]>(`/nodes/${id}/attributes`);
}

async function updateNodeAttribute(id: number, key: string, value: string): Promise<void> {
    return push(`/nodes/${id}/attributes`, { key, value });
}

// ============= Components =============

const CategoryList: React.FC<{
    categories: Category[];
    loading: boolean;
    onEdit: (category: Category) => void;
    onDelete: (id: number) => void;
}> = ({ categories, loading, onEdit, onDelete }) => {
    return (
        <Table.Root variant="surface">
            <Table.Header>
                <Table.Row>
                    <Table.ColumnHeaderCell>ID</Table.ColumnHeaderCell>
                    <Table.ColumnHeaderCell style={{ minWidth: "200px" }}>名称</Table.ColumnHeaderCell>
                    <Table.ColumnHeaderCell style={{ minWidth: "120px" }}>Slug</Table.ColumnHeaderCell>
                    <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>首页显示</Table.ColumnHeaderCell>
                    <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>访问权限</Table.ColumnHeaderCell>
                    <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>操作</Table.ColumnHeaderCell>
                </Table.Row>
            </Table.Header>

            <Table.Body>
                {categories.map((category) => (
                    <Table.Row key={category.id}>
                        <Table.Cell>{category.id}</Table.Cell>
                        <Table.Cell>
                            <Flex direction="column">
                                <Text weight="medium">{category.name}</Text>
                                <Text size="1" color="gray">
                                    {category.description}
                                </Text>
                            </Flex>
                        </Table.Cell>
                        <Table.Cell>
                            <Text color="gray">{category.slug}</Text>
                        </Table.Cell>
                        <Table.Cell>
                            <Text color={category.show_in_list ? "green" : "gray"}>
                                {category.show_in_list ? "是" : "否"}
                            </Text>
                        </Table.Cell>
                        <Table.Cell>
                            <Flex direction="column" gap="1">
                                {category.moderator_access_required && (
                                    <Text size="1" color="amber" weight="bold">管理员</Text>
                                )}
                                {category.member_access_required && (
                                    <Text size="1" color="blue">会员</Text>
                                )}
                                {!category.moderator_access_required && !category.member_access_required && (
                                    <Text size="1" color="gray">公开</Text>
                                )}
                                {category.isolated && (
                                    <Text size="1" color="purple">隔离</Text>
                                )}
                                {category.access_only && (
                                    <Text size="1" color="red">只读</Text>
                                )}
                            </Flex>
                        </Table.Cell>
                        <Table.Cell>
                            <Flex gap="2">
                                <Button
                                    size="1"
                                    variant="soft"
                                    onClick={() => onEdit(category)}
                                    disabled={loading}
                                >
                                    编辑
                                </Button>
                                <Button
                                    size="1"
                                    variant="soft"
                                    color="red"
                                    onClick={() => onDelete(category.id)}
                                    disabled={loading}
                                >
                                    删除
                                </Button>
                            </Flex>
                        </Table.Cell>
                    </Table.Row>
                ))}
            </Table.Body>
        </Table.Root>
    );
};

const CategoryCreateDialog: React.FC<{
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onSuccess: () => void;
}> = ({ open, onOpenChange, onSuccess }) => {
    const [form, setForm] = useState<CategoryCreateParams>({
        name: "",
        slug: "",
        description: "",
        show_in_list: true,
        member_access_required: false,
        moderator_access_required: false,
        isolated: false,
        access_only: false,
        topic_reward: 0,
        comment_reward: 0,
    });

    const handleCreate = async () => {
        if (!form.name || !form.slug) {
            alert("请填写分类名称和 Slug");
            return;
        }

        try {
            await createCategory(form);
            onOpenChange(false);
            setForm({
                name: "",
                slug: "",
                description: "",
                show_in_list: true,
                member_access_required: false,
                moderator_access_required: false,
                isolated: false,
                access_only: false,
                topic_reward: 0,
                comment_reward: 0,
            });
            onSuccess();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "创建分类失败";
            alert(errorMessage);
        }
    };

    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Content style={{ width: "100%", maxWidth: 500 }}>
                <Dialog.Title>创建新分类</Dialog.Title>
                <Dialog.Description size="2" mb="4">
                    填写分类信息
                </Dialog.Description>

                <Tabs.Root defaultValue="basic">
                    <Tabs.List>
                        <Tabs.Trigger value="basic">基本信息</Tabs.Trigger>
                        <Tabs.Trigger value="permissions">权限与设置</Tabs.Trigger>
                        <Tabs.Trigger value="rewards">奖励</Tabs.Trigger>
                    </Tabs.List>

                    <Box pt="3">
                        <Tabs.Content value="basic">
                            <Flex direction="column" gap="3">
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        分类名称 *
                                    </Text>
                                    <TextField.Root
                                        placeholder="输入分类名称"
                                        value={form.name}
                                        onChange={(e) => setForm({ ...form, name: e.target.value })}
                                    />
                                </Box>

                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        Slug *
                                    </Text>
                                    <TextField.Root
                                        placeholder="输入 URL 友好的标识符"
                                        value={form.slug}
                                        onChange={(e) => setForm({ ...form, slug: e.target.value })}
                                    />
                                </Box>

                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        分类描述
                                    </Text>
                                    <TextArea
                                        placeholder="输入分类描述"
                                        rows={3}
                                        value={form.description}
                                        onChange={(e) => setForm({ ...form, description: e.target.value })}
                                    />
                                </Box>

                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.show_in_list}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, show_in_list: checked === true })
                                            }
                                        />
                                        <Text size="2">在首页列表中显示</Text>
                                    </Flex>
                                </Box>
                            </Flex>
                        </Tabs.Content>

                        <Tabs.Content value="permissions">
                            <Flex direction="column" gap="3">
                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.member_access_required}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, member_access_required: checked === true })
                                            }
                                        />
                                        <Text size="2">仅允许会员访问</Text>
                                    </Flex>
                                </Box>
                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.moderator_access_required}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, moderator_access_required: checked === true })
                                            }
                                        />
                                        <Text size="2">仅允许管理员访问</Text>
                                    </Flex>
                                </Box>
                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.isolated}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, isolated: checked === true })
                                            }
                                        />
                                        <Text size="2">隔离节点（用户仅能看到自己的帖子）</Text>
                                    </Flex>
                                </Box>
                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.access_only}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, access_only: checked === true })
                                            }
                                        />
                                        <Text size="2">只读节点（禁止发帖）</Text>
                                    </Flex>
                                </Box>
                            </Flex>
                        </Tabs.Content>

                        <Tabs.Content value="rewards">
                            <Flex direction="column" gap="3">
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        发帖奖励（负数表示消耗）
                                    </Text>
                                    <TextField.Root
                                        type="number"
                                        placeholder="0"
                                        value={form.topic_reward}
                                        onChange={(e) =>
                                            setForm({
                                                ...form,
                                                topic_reward: parseInt(e.target.value) || 0,
                                            })
                                        }
                                    />
                                </Box>
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        回复奖励（负数表示消耗）
                                    </Text>
                                    <TextField.Root
                                        type="number"
                                        placeholder="0"
                                        value={form.comment_reward}
                                        onChange={(e) =>
                                            setForm({
                                                ...form,
                                                comment_reward: parseInt(e.target.value) || 0,
                                            })
                                        }
                                    />
                                </Box>
                            </Flex>
                        </Tabs.Content>

                    </Box>
                </Tabs.Root>

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray">
                            取消
                        </Button>
                    </Dialog.Close>
                    <Button onClick={handleCreate}>确认创建</Button>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};


const CategoryEditDialog: React.FC<{
    open: boolean;
    onOpenChange: (open: boolean) => void;
    category: Category | null;
    onSuccess: () => void;
}> = ({ open, onOpenChange, category, onSuccess }) => {
    const [form, setForm] = useState<CategoryUpdateParams>({
        name: "",
        slug: "",
        description: "",
        show_in_list: true,
        member_access_required: false,
        moderator_access_required: false,
        isolated: false,
        access_only: false,
        topic_reward: 0,
        comment_reward: 0,
    });

    const [iconImage, setIconImage] = useState("");
    const [customHtml, setCustomHtml] = useState("");
    const [attrLoading, setAttrLoading] = useState(false);

    useEffect(() => {
        if (category) {
            setForm({
                name: category.name,
                slug: category.slug,
                description: category.description,
                show_in_list: category.show_in_list,
                member_access_required: category.member_access_required,
                moderator_access_required: category.moderator_access_required,
                isolated: category.isolated || false,
                access_only: category.access_only || false,
                topic_reward: category.topic_reward || 0,
                comment_reward: category.comment_reward || 0,
            });
        }
    }, [category]);

    useEffect(() => {
        if (open && category) {
            setAttrLoading(true);
            getNodeAttributes(category.id)
                .then((attrs) => {
                    const attrMap = new Map(attrs);
                    setIconImage(attrMap.get("icon_image") || "");
                    setCustomHtml(attrMap.get("custom_html") || "");
                })
                .catch(() => {
                    setIconImage("");
                    setCustomHtml("");
                })
                .finally(() => setAttrLoading(false));
        } else {
            setIconImage("");
            setCustomHtml("");
        }
    }, [open, category]);

    const handleUpdate = async () => {
        if (!category || !form.name || !form.slug) {
            alert("请填写分类名称和 Slug");
            return;
        }

        try {
            await updateCategory(category.id, form);
            // 保存自定义属性
            await updateNodeAttribute(category.id, "icon_image", iconImage);
            await updateNodeAttribute(category.id, "custom_html", customHtml);
            onOpenChange(false);
            onSuccess();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "更新分类失败";
            alert(errorMessage);
        }
    };

    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Content style={{ width: "100%", maxWidth: 500 }}>
                <Dialog.Title>编辑分类</Dialog.Title>
                <Dialog.Description size="2" mb="4">
                    修改分类信息
                </Dialog.Description>

                <Tabs.Root defaultValue="basic">
                    <Tabs.List>
                        <Tabs.Trigger value="basic">基本信息</Tabs.Trigger>
                        <Tabs.Trigger value="permissions">权限与设置</Tabs.Trigger>
                        <Tabs.Trigger value="rewards">奖励</Tabs.Trigger>
                        <Tabs.Trigger value="appearance">外观</Tabs.Trigger>
                    </Tabs.List>

                    <Box pt="3">
                        <Tabs.Content value="basic">
                            <Flex direction="column" gap="3">
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        分类名称 *
                                    </Text>
                                    <TextField.Root
                                        placeholder="输入分类名称"
                                        value={form.name}
                                        onChange={(e) => setForm({ ...form, name: e.target.value })}
                                    />
                                </Box>

                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        Slug *
                                    </Text>
                                    <TextField.Root
                                        placeholder="输入 URL 友好的标识符"
                                        value={form.slug}
                                        onChange={(e) => setForm({ ...form, slug: e.target.value })}
                                    />
                                </Box>

                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        分类描述
                                    </Text>
                                    <TextArea
                                        placeholder="输入分类描述"
                                        rows={3}
                                        value={form.description}
                                        onChange={(e) => setForm({ ...form, description: e.target.value })}
                                    />
                                </Box>

                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.show_in_list}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, show_in_list: checked === true })
                                            }
                                        />
                                        <Text size="2">在首页列表中显示</Text>
                                    </Flex>
                                </Box>
                            </Flex>
                        </Tabs.Content>

                        <Tabs.Content value="permissions">
                            <Flex direction="column" gap="3">
                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.member_access_required}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, member_access_required: checked === true })
                                            }
                                        />
                                        <Text size="2">仅允许会员访问</Text>
                                    </Flex>
                                </Box>
                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.moderator_access_required}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, moderator_access_required: checked === true })
                                            }
                                        />
                                        <Text size="2">仅允许管理员访问</Text>
                                    </Flex>
                                </Box>
                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.isolated}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, isolated: checked === true })
                                            }
                                        />
                                        <Text size="2">隔离节点（用户仅能看到自己的帖子）</Text>
                                    </Flex>
                                </Box>
                                <Box>
                                    <Flex align="center" gap="2">
                                        <Checkbox
                                            checked={form.access_only}
                                            onCheckedChange={(checked) =>
                                                setForm({ ...form, access_only: checked === true })
                                            }
                                        />
                                        <Text size="2">只读节点（禁止发帖）</Text>
                                    </Flex>
                                </Box>
                            </Flex>
                        </Tabs.Content>

                        <Tabs.Content value="rewards">
                            <Flex direction="column" gap="3">
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        发帖奖励（负数表示消耗）
                                    </Text>
                                    <TextField.Root
                                        type="number"
                                        placeholder="0"
                                        value={form.topic_reward}
                                        onChange={(e) =>
                                            setForm({ ...form, topic_reward: parseInt(e.target.value) || 0 })
                                        }
                                    />
                                </Box>
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        回复奖励（负数表示消耗）
                                    </Text>
                                    <TextField.Root
                                        type="number"
                                        placeholder="0"
                                        value={form.comment_reward}
                                        onChange={(e) =>
                                            setForm({ ...form, comment_reward: parseInt(e.target.value) || 0 })
                                        }
                                    />
                                </Box>
                            </Flex>
                        </Tabs.Content>

                        <Tabs.Content value="appearance">
                            <Flex direction="column" gap="3">
                                {attrLoading && (
                                    <Text size="2" color="gray">加载属性中...</Text>
                                )}
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        节点图标地址
                                    </Text>
                                    <TextField.Root
                                        placeholder="输入图标图片 URL"
                                        value={iconImage}
                                        onChange={(e) => setIconImage(e.target.value)}
                                    />
                                    <Text size="1" color="gray" mt="1">
                                        节点的图标图片链接，留空则不显示
                                    </Text>
                                </Box>
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        自定义 HTML 代码
                                    </Text>
                                    <TextArea
                                        placeholder="输入自定义 HTML 代码"
                                        rows={5}
                                        value={customHtml}
                                        onChange={(e) => setCustomHtml(e.target.value)}
                                    />
                                    <Text size="1" color="gray" mt="1">
                                        可在节点页面中嵌入自定义 HTML 内容
                                    </Text>
                                </Box>
                            </Flex>
                        </Tabs.Content>

                    </Box>
                </Tabs.Root>

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray">
                            取消
                        </Button>
                    </Dialog.Close>
                    <Button onClick={handleUpdate}>确认更新</Button>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};


const NodePage: React.FC = () => {
    const [categories, setCategories] = useState<Category[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Dialog Data states
    const [createDialogOpen, setCreateDialogOpen] = useState(false);
    const [editDialogOpen, setEditDialogOpen] = useState(false);
    const [editingCategory, setEditingCategory] = useState<Category | null>(null);

    const loadCategories = async () => {
        setLoading(true);
        setError(null);
        try {
            const data = await getCategories();
            setCategories(data);
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "加载分类列表失败";
            setError(errorMessage);
            console.error("Failed to load categories:", err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadCategories();
    }, []);

    const handleEditClick = (category: Category) => {
        setEditingCategory(category);
        setEditDialogOpen(true);
    };

    const handleDelete = async (categoryId: number) => {
        if (!confirm("确定要删除这个分类吗？这可能会影响相关的帖子。")) return;

        try {
            await deleteCategory(categoryId);
            await loadCategories();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "删除分类失败";
            alert(errorMessage);
        }
    };

    return (
        <Container size="4">
            <Flex direction="column" gap="5">
                {/* Header */}
                <Flex align="center" justify="between">
                    <Box>
                        <Heading as="h1" size="6" mb="2">
                            分类管理
                        </Heading>
                        <Text as="p" size="3" color="gray">
                            管理论坛分类和板块
                        </Text>
                    </Box>
                    <Button size="3" onClick={() => setCreateDialogOpen(true)}>创建新分类</Button>
                </Flex>

                {/* Error Message */}
                {error && (
                    <Card>
                        <Text color="red" size="2">
                            错误: {error}
                        </Text>
                    </Card>
                )}

                {/* Summary */}
                <Text size="2" color="gray">
                    共 {categories.length} 个分类{loading && " (加载中...)"}
                </Text>

                {/* Categories Table */}
                <CategoryList
                    categories={categories}
                    loading={loading}
                    onEdit={handleEditClick}
                    onDelete={handleDelete}
                />

                {categories.length === 0 && !loading && (
                    <Card>
                        <Box p="4" style={{ textAlign: "center" }}>
                            <Text color="gray">暂无数据</Text>
                        </Box>
                    </Card>
                )}

                {/* Info Card */}
                <Card>
                    <Flex direction="column" gap="2">
                        <Text size="2" weight="bold">
                            💡 提示
                        </Text>
                        <Text size="2" color="gray">
                            分类用于组织和管理论坛中的帖子。请谨慎删除分类，以免影响现有帖子的分类归属。
                        </Text>
                    </Flex>
                </Card>

                <CategoryCreateDialog
                    open={createDialogOpen}
                    onOpenChange={setCreateDialogOpen}
                    onSuccess={loadCategories}
                />

                <CategoryEditDialog
                    open={editDialogOpen}
                    onOpenChange={setEditDialogOpen}
                    category={editingCategory}
                    onSuccess={loadCategories}
                />
            </Flex>
        </Container>
    );
};

export default NodePage;
