import React, { useEffect, useState } from "react";
import {
    Box,
    Button,
    Card,
    Container,
    Dialog,
    Flex,
    Heading,
    IconButton,
    Separator,
    Switch,
    Table,
    Text,
    TextArea,
    TextField,
    Tooltip,
} from "@radix-ui/themes";
import {
    ArrowDownIcon,
    ArrowUpIcon,
    ChevronDownIcon,
    ChevronRightIcon,
    Pencil1Icon,
    PlusIcon,
    TrashIcon,
} from "@radix-ui/react-icons";
import { pull, push, SysError } from "../api";

// 类型定义
interface Link {
    title: string;
    url: string;
    description: string;
    blank: boolean;
}

interface LinkCollection {
    title: string;
    links: Link[];
}

type SiteBottomLinks = LinkCollection[];

const load = () => pull<SiteBottomLinks>(`/settings?name=site_bottom_links`);
const save = (value: SiteBottomLinks) =>
    push(`/settings`, { name: "site_bottom_links", value });

// --- Components ---

// 1. Link Edit Dialog
interface LinkEditDialogProps {
    initialLink?: Link;
    message?: string; // Trigger button text or specialized trigger
    onSave: (link: Link) => void;
    trigger?: React.ReactNode;
}

const LinkEditDialog: React.FC<LinkEditDialogProps> = ({
    initialLink,
    onSave,
    trigger,
}) => {
    const [open, setOpen] = useState(false);
    const [link, setLink] = useState<Link>(
        initialLink || { title: "", url: "", description: "", blank: false }
    );

    useEffect(() => {
        if (open) {
            setLink(
                initialLink || { title: "", url: "", description: "", blank: false }
            );
        }
    }, [open, initialLink]);

    const handleSave = () => {
        onSave(link);
        setOpen(false);
    };

    return (
        <Dialog.Root open={open} onOpenChange={setOpen}>
            <Dialog.Trigger>
                {trigger || (
                    <Button variant="soft" size="1">
                        <PlusIcon /> 添加链接
                    </Button>
                )}
            </Dialog.Trigger>

            <Dialog.Content style={{ maxWidth: 450 }}>
                <Dialog.Title>
                    {initialLink ? "编辑链接" : "添加新链接"}
                </Dialog.Title>

                <Flex direction="column" gap="3">
                    <Box>
                        <Text as="div" size="2" mb="1" weight="bold">
                            链接标题
                        </Text>
                        <TextField.Root
                            value={link.title}
                            onChange={(e) => setLink({ ...link, title: e.target.value })}
                            placeholder="例如: 帮助中心"
                        />
                    </Box>
                    <Box>
                        <Text as="div" size="2" mb="1" weight="bold">
                            链接地址
                        </Text>
                        <TextField.Root
                            value={link.url}
                            onChange={(e) => setLink({ ...link, url: e.target.value })}
                            placeholder="https://example.com"
                        />
                    </Box>
                    <Box>
                        <Text as="div" size="2" mb="1" weight="bold">
                            描述 (鼠标悬浮提示)
                        </Text>
                        <TextArea
                            value={link.description}
                            onChange={(e) =>
                                setLink({ ...link, description: e.target.value })
                            }
                            placeholder="可选描述..."
                        />
                    </Box>
                    <Flex align="center" gap="2">
                        <Switch
                            checked={link.blank}
                            onCheckedChange={(checked) =>
                                setLink({ ...link, blank: checked })
                            }
                        />
                        <Text size="2">在新标签页打开</Text>
                    </Flex>
                </Flex>

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray">
                            取消
                        </Button>
                    </Dialog.Close>
                    <Button onClick={handleSave}>保存</Button>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};

// 2. Collection Item (Accordion)
interface CollectionItemProps {
    collection: LinkCollection;
    index: number;
    total: number;
    onUpdateTitle: (newTitle: string) => void;
    onRemove: () => void;
    onMove: (direction: "up" | "down") => void;
    onUpdateLinks: (newLinks: Link[]) => void;
}

const CollectionItem: React.FC<CollectionItemProps> = ({
    collection,
    index,
    total,
    onUpdateTitle,
    onRemove,
    onMove,
    onUpdateLinks,
}) => {
    const [expanded, setExpanded] = useState(true);

    const handleAddLink = (newLink: Link) => {
        onUpdateLinks([...collection.links, newLink]);
    };

    const handleEditLink = (linkIndex: number, updatedLink: Link) => {
        const newLinks = [...collection.links];
        newLinks[linkIndex] = updatedLink;
        onUpdateLinks(newLinks);
    };

    const handleRemoveLink = (linkIndex: number) => {
        const newLinks = collection.links.filter((_, i) => i !== linkIndex);
        onUpdateLinks(newLinks);
    };

    const handleMoveLink = (linkIndex: number, direction: "up" | "down") => {
        const newIndex = direction === "up" ? linkIndex - 1 : linkIndex + 1;
        if (newIndex < 0 || newIndex >= collection.links.length) return;

        const newLinks = [...collection.links];
        [newLinks[linkIndex], newLinks[newIndex]] = [
            newLinks[newIndex],
            newLinks[linkIndex],
        ];
        onUpdateLinks(newLinks);
    };

    return (
        <Card style={{ padding: 0, overflow: "hidden" }}>
            {/* Header Bar */}
            <Flex
                align="center"
                justify="between"
                style={{
                    padding: "12px 16px",
                    backgroundColor: "var(--gray-3)",
                    cursor: "pointer",
                }}
                onClick={(e) => {
                    // Only toggle if not clicking interactive elements
                    if ((e.target as HTMLElement).tagName !== 'INPUT' && (e.target as HTMLElement).tagName !== 'BUTTON') {
                        setExpanded(!expanded);
                    }
                }}
            >
                <Flex align="center" gap="3" style={{ flex: 1 }}>
                    <IconButton
                        variant="ghost"
                        color="gray"
                        size="1"
                        onClick={(e) => {
                            e.stopPropagation();
                            setExpanded(!expanded);
                        }}
                    >
                        {expanded ? <ChevronDownIcon /> : <ChevronRightIcon />}
                    </IconButton>

                    <Box style={{ width: 300 }} onClick={(e) => e.stopPropagation()}>
                        <TextField.Root
                            value={collection.title}
                            onChange={(e) => onUpdateTitle(e.target.value)}
                            placeholder="分组名称"
                            variant="surface"
                        />
                    </Box>
                    <Text size="2" color="gray">
                        ({collection.links.length} 个链接)
                    </Text>
                </Flex>

                <Flex gap="1" onClick={(e) => e.stopPropagation()}>
                    <Tooltip content="上移分组">
                        <IconButton
                            variant="ghost"
                            disabled={index === 0}
                            onClick={() => onMove("up")}
                        >
                            <ArrowUpIcon />
                        </IconButton>
                    </Tooltip>
                    <Tooltip content="下移分组">
                        <IconButton
                            variant="ghost"
                            disabled={index === total - 1}
                            onClick={() => onMove("down")}
                        >
                            <ArrowDownIcon />
                        </IconButton>
                    </Tooltip>
                    <Tooltip content="删除分组">
                        <IconButton variant="ghost" color="red" onClick={onRemove}>
                            <TrashIcon />
                        </IconButton>
                    </Tooltip>
                </Flex>
            </Flex>

            {/* Body */}
            {expanded && (
                <Box p="4">
                    <Table.Root variant="surface">
                        <Table.Header>
                            <Table.Row>
                                <Table.ColumnHeaderCell>标题</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell>URL</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell width="100px">
                                    新标签页
                                </Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell width="120px" align="right">
                                    操作
                                </Table.ColumnHeaderCell>
                            </Table.Row>
                        </Table.Header>

                        <Table.Body>
                            {collection.links.map((link, i) => (
                                <Table.Row key={i}>
                                    <Table.Cell>
                                        <Flex gap="2" align="center">
                                            <Text weight="medium">{link.title}</Text>
                                            {link.description && (
                                                <Tooltip content={link.description}>
                                                    <Text size="1" color="gray" style={{ cursor: 'help' }}>[?]</Text>
                                                </Tooltip>
                                            )}
                                        </Flex>
                                    </Table.Cell>
                                    <Table.Cell>
                                        <Text size="2" color="gray" style={{ wordBreak: 'break-all' }}>
                                            {link.url}
                                        </Text>
                                    </Table.Cell>
                                    <Table.Cell>
                                        {link.blank ? "是" : "否"}
                                    </Table.Cell>
                                    <Table.Cell align="right">
                                        <Flex gap="2" justify="end">
                                            <LinkEditDialog
                                                initialLink={link}
                                                onSave={(updated) => handleEditLink(i, updated)}
                                                trigger={
                                                    <IconButton variant="ghost" size="1">
                                                        <Pencil1Icon />
                                                    </IconButton>
                                                }
                                            />
                                            <IconButton
                                                variant="ghost"
                                                size="1"
                                                disabled={i === 0}
                                                onClick={() => handleMoveLink(i, "up")}
                                            >
                                                <ArrowUpIcon />
                                            </IconButton>
                                            <IconButton
                                                variant="ghost"
                                                size="1"
                                                disabled={i === collection.links.length - 1}
                                                onClick={() => handleMoveLink(i, "down")}
                                            >
                                                <ArrowDownIcon />
                                            </IconButton>
                                            <IconButton
                                                variant="ghost"
                                                color="red"
                                                size="1"
                                                onClick={() => handleRemoveLink(i)}
                                            >
                                                <TrashIcon />
                                            </IconButton>
                                        </Flex>
                                    </Table.Cell>
                                </Table.Row>
                            ))}
                            {collection.links.length === 0 && (
                                <Table.Row>
                                    <Table.Cell colSpan={4} align="center">
                                        <Text color="gray">暂无链接</Text>
                                    </Table.Cell>
                                </Table.Row>
                            )}
                        </Table.Body>
                    </Table.Root>

                    <Box mt="3">
                        <LinkEditDialog onSave={handleAddLink} />
                    </Box>
                </Box>
            )}
        </Card>
    );
};

// --- Main Page ---

const LinksPage: React.FC = () => {
    const [loading, setLoading] = useState(false);
    const [saving, setSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [linkCollections, setLinkCollections] = useState<SiteBottomLinks>([]);

    const loadSettings = async () => {
        setLoading(true);
        setError(null);
        try {
            const data = await load();
            setLinkCollections(data || []);
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "加载链接配置失败";
            setError(errorMessage);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadSettings();
    }, []);

    const handleSave = async () => {
        setSaving(true);
        try {
            await save(linkCollections);
            alert("链接配置保存成功");
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存链接配置失败";
            alert(errorMessage);
        } finally {
            setSaving(false);
        }
    };

    const addCollection = () => {
        setLinkCollections([
            ...linkCollections,
            { title: "新分组", links: [] },
        ]);
    };

    const updateCollection = (index: number, newCollection: LinkCollection) => {
        const updated = [...linkCollections];
        updated[index] = newCollection;
        setLinkCollections(updated);
    }

    const removeCollection = (index: number) => {
        if (!confirm("确定要删除这个分组吗？")) return;
        const updated = linkCollections.filter((_, i) => i !== index);
        setLinkCollections(updated);
    };

    const moveCollection = (index: number, direction: "up" | "down") => {
        const newIndex = direction === "up" ? index - 1 : index + 1;
        if (newIndex < 0 || newIndex >= linkCollections.length) return;
        const updated = [...linkCollections];
        [updated[index], updated[newIndex]] = [updated[newIndex], updated[index]];
        setLinkCollections(updated);
    };

    return (
        <Container size="4">
            <Flex direction="column" gap="5">
                {/* Header */}
                <Flex justify="between" align="center">
                    <Box>
                        <Heading as="h1" size="6" mb="2">
                            页面导航链接
                        </Heading>
                        <Text as="p" size="3" color="gray">
                            管理网站底部的导航链接
                        </Text>
                    </Box>
                    <Flex gap="2">
                        <Button variant="outline" onClick={addCollection}>
                            <PlusIcon /> 添加分组
                        </Button>
                        <Button onClick={handleSave} disabled={saving}>
                            {saving ? "保存中..." : "保存配置"}
                        </Button>
                    </Flex>
                </Flex>

                {error && (
                    <Card>
                        <Text color="red">{error}</Text>
                    </Card>
                )}

                {loading ? (
                    <Card><Text>加载中...</Text></Card>
                ) : (
                    <Flex direction="column" gap="4">
                        {linkCollections.map((collection, index) => (
                            <CollectionItem
                                key={index}
                                index={index}
                                total={linkCollections.length}
                                collection={collection}
                                onUpdateTitle={(title) => updateCollection(index, { ...collection, title })}
                                onRemove={() => removeCollection(index)}
                                onMove={(dir) => moveCollection(index, dir)}
                                onUpdateLinks={(links) => updateCollection(index, { ...collection, links })}
                            />
                        ))}
                        {linkCollections.length === 0 && (
                            <Card>
                                <Flex justify="center" p="5">
                                    <Text color="gray">暂无链接分组</Text>
                                </Flex>
                            </Card>
                        )}
                    </Flex>
                )}
            </Flex>
        </Container>
    );
};

export default LinksPage;
