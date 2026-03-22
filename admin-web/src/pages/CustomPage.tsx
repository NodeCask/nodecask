import React, {useEffect, useState} from "react";
import {
    Box,
    Button,
    Card,
    Container,
    Dialog,
    Flex,
    Heading,
    Select,
    Table,
    Text,
    TextArea,
    TextField,
} from "@radix-ui/themes";
import {pull, push, SysError} from "../api";

export interface PageType {
    id: number;
    path: string;
    title: string;
    description: string;
    content_type: string;
    content: string;
    created_at: string;
    updated_at: string;
}

export interface CustomPageForm {
    path: string;
    title: string;
    description: string;
    content_type: string;
    content: string;
}

const getCustomPages = () => pull<PageType[]>("/pages");
const createCustomPage = (data: CustomPageForm) => push("/pages", data);
const updateCustomPage = (id: number, data: CustomPageForm) => push(`/pages/${id}`, data);
const deleteCustomPage = (id: number) => push(`/pages/${id}/delete`);

const CustomPage: React.FC = () => {
    const [pages, setPages] = useState<PageType[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Create Dialog State
    const [createDialogOpen, setCreateDialogOpen] = useState(false);
    const [createForm, setCreateForm] = useState<CustomPageForm>({
        path: "",
        title: "",
        description: "",
        content_type: "markdown",
        content: "",
    });

    // Edit Dialog State
    const [editDialogOpen, setEditDialogOpen] = useState(false);
    const [editingPage, setEditingPage] = useState<PageType | null>(null);
    const [editForm, setEditForm] = useState<CustomPageForm>({
        path: "/",
        title: "",
        description: "",
        content_type: "markdown",
        content: "",
    });

    // Load Pages
    const loadPages = async () => {
        setLoading(true);
        setError(null);
        try {
            const data = await getCustomPages();
            setPages(data);
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "Failed to load pages";
            setError(errorMessage);
            console.error("Failed to load pages:", err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadPages();
    }, []);

    // Create Page
    const handleCreate = async () => {
        if (!createForm.path || !createForm.title || !createForm.content) {
            alert("Please fill in required fields (Slug, Title, Content)");
            return;
        }

        try {
            await createCustomPage(createForm);
            setCreateDialogOpen(false);
            setCreateForm({
                path: "",
                title: "",
                description: "",
                content_type: "markdown",
                content: "",
            });
            await loadPages();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "Failed to create page";
            alert(errorMessage);
        }
    };

    // Open Edit Dialog
    const handleEditClick = (page: PageType) => {
        setEditingPage(page);
        setEditForm({
            path: page.path,
            title: page.title,
            description: page.description,
            content_type: page.content_type,
            content: page.content,
        });
        setEditDialogOpen(true);
    };

    // Update Page
    const handleUpdate = async () => {
        if (!editingPage || !editForm.path || !editForm.title || !editForm.content) {
            alert("Please fill in required fields (Slug, Title, Content)");
            return;
        }

        try {
            await updateCustomPage(editingPage.id, editForm);
            setEditDialogOpen(false);
            setEditingPage(null);
            await loadPages();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "Failed to update page";
            alert(errorMessage);
        }
    };

    // Delete Page
    const handleDelete = async (id: number) => {
        if (!confirm("Are you sure you want to delete this page?")) return;

        try {
            await deleteCustomPage(id);
            await loadPages();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "Failed to delete page";
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
                            自定义页面
                        </Heading>
                        <Text as="p" size="3" color="gray">
                            管理站点自定义页面 (如: 关于我们, 隐私政策)
                        </Text>
                    </Box>
                    <Dialog.Root open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
                        <Dialog.Trigger>
                            <Button size="3">创建新页面</Button>
                        </Dialog.Trigger>
                        <Dialog.Content style={{maxWidth: 800}}>
                            <Dialog.Title>创建新页面</Dialog.Title>
                            <Dialog.Description size="2" mb="4">
                                填写页面信息
                            </Dialog.Description>

                            <Flex direction="column" gap="3">
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        Slug (URL 路径) *
                                    </Text>
                                    <TextField.Root
                                        placeholder="例如: about, privacy"
                                        value={createForm.path}
                                        onChange={(e) =>
                                            setCreateForm({...createForm, path: e.target.value})
                                        }
                                    />
                                </Box>

                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        标题 *
                                    </Text>
                                    <TextField.Root
                                        placeholder="页面标题"
                                        value={createForm.title}
                                        onChange={(e) =>
                                            setCreateForm({...createForm, title: e.target.value})
                                        }
                                    />
                                </Box>

                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        描述 (SEO)
                                    </Text>
                                    <TextArea
                                        placeholder="页面描述"
                                        rows={2}
                                        value={createForm.description}
                                        onChange={(e) =>
                                            setCreateForm({
                                                ...createForm,
                                                description: e.target.value,
                                            })
                                        }
                                    />
                                </Box>

                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        内容类型
                                    </Text>
                                    <Select.Root
                                        value={createForm.content_type}
                                        onValueChange={(value) =>
                                            setCreateForm({...createForm, content_type: value})
                                        }
                                    >
                                        <Select.Trigger/>
                                        <Select.Content>
                                            <Select.Item value="markdown">Markdown</Select.Item>
                                            <Select.Item value="html">HTML</Select.Item>
                                        </Select.Content>
                                    </Select.Root>
                                </Box>

                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="1">
                                        内容 *
                                    </Text>
                                    <TextArea
                                        placeholder="页面内容..."
                                        rows={15}
                                        style={{fontFamily: "monospace"}}
                                        value={createForm.content}
                                        onChange={(e) =>
                                            setCreateForm({...createForm, content: e.target.value})
                                        }
                                    />
                                </Box>
                            </Flex>

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
                </Flex>

                {/* Error Message */}
                {error && (
                    <Card>
                        <Text color="red" size="2">
                            错误: {error}
                        </Text>
                    </Card>
                )}

                {/* Page List */}
                <Table.Root variant="surface">
                    <Table.Header>
                        <Table.Row>
                            <Table.ColumnHeaderCell>ID</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>Path</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>标题</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>类型</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>更新时间</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>操作</Table.ColumnHeaderCell>
                        </Table.Row>
                    </Table.Header>

                    <Table.Body>
                        {pages.map((page) => (
                            <Table.Row key={page.id}>
                                <Table.Cell>{page.id}</Table.Cell>
                                <Table.Cell>
                                    <Text weight="medium">{page.path}</Text>
                                </Table.Cell>
                                <Table.Cell>{page.title}</Table.Cell>
                                <Table.Cell>{page.content_type}</Table.Cell>
                                <Table.Cell>
                                    {new Date(page.updated_at).toLocaleString()}
                                </Table.Cell>
                                <Table.Cell>
                                    <Flex gap="2">
                                        <Button
                                            size="1"
                                            variant="soft"
                                            onClick={() => handleEditClick(page)}
                                            disabled={loading}
                                        >
                                            编辑
                                        </Button>
                                        <Button
                                            size="1"
                                            variant="soft"
                                            color="red"
                                            onClick={() => handleDelete(page.id)}
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

                {pages.length === 0 && !loading && (
                    <Card>
                        <Box p="4" style={{textAlign: "center"}}>
                            <Text color="gray">暂无自定义页面</Text>
                        </Box>
                    </Card>
                )}

                {/* Edit Dialog */}
                <Dialog.Root open={editDialogOpen} onOpenChange={setEditDialogOpen}>
                    <Dialog.Content style={{maxWidth: 800}}>
                        <Dialog.Title>编辑页面</Dialog.Title>
                        <Dialog.Description size="2" mb="4">
                            修改页面内容
                        </Dialog.Description>

                        <Flex direction="column" gap="3">
                            <Box>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    Path (URL 路径，以 / 开头) *
                                </Text>
                                <TextField.Root
                                    placeholder="例如: /about"
                                    value={editForm.path}
                                    onChange={(e) =>
                                        setEditForm({...editForm, path: e.target.value})
                                    }
                                />
                            </Box>

                            <Box>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    标题 *
                                </Text>
                                <TextField.Root
                                    placeholder="页面标题"
                                    value={editForm.title}
                                    onChange={(e) =>
                                        setEditForm({...editForm, title: e.target.value})
                                    }
                                />
                            </Box>

                            <Box>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    描述 (SEO)
                                </Text>
                                <TextArea
                                    placeholder="页面描述"
                                    rows={2}
                                    value={editForm.description}
                                    onChange={(e) =>
                                        setEditForm({
                                            ...editForm,
                                            description: e.target.value,
                                        })
                                    }
                                />
                            </Box>

                            <Box>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    内容类型
                                </Text>
                                <Select.Root
                                    value={editForm.content_type}
                                    onValueChange={(value) =>
                                        setEditForm({...editForm, content_type: value})
                                    }
                                >
                                    <Select.Trigger/>
                                    <Select.Content>
                                        <Select.Item value="markdown">Markdown</Select.Item>
                                        <Select.Item value="html">HTML</Select.Item>
                                    </Select.Content>
                                </Select.Root>
                            </Box>

                            <Box>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    内容 *
                                </Text>
                                <TextArea
                                    placeholder="页面内容..."
                                    rows={15}
                                    style={{fontFamily: "monospace"}}
                                    value={editForm.content}
                                    onChange={(e) =>
                                        setEditForm({...editForm, content: e.target.value})
                                    }
                                />
                            </Box>
                        </Flex>

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
            </Flex>
        </Container>
    );
};

export default CustomPage;
