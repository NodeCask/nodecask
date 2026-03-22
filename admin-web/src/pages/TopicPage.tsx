import React, {useEffect, useState} from "react";
import {
    Badge,
    Box,
    Button,
    Card,
    Container,
    Flex,
    Grid,
    Heading,
    Select,
    Table,
    Text,
    TextField,
} from "@radix-ui/themes";
import Pagination from "../components/Pagination";
import {type PaginationParams, type PaginationResponse, pull, push, SysError} from "../api";
import type {Category} from "./NodePage.tsx";

export interface Post {
    id: number;
    title: string;
    content: string;
    user_id: number;
    username: string;
    node_id: number;
    node_name: string;
    node_slug: string;
    view_count: number;
    is_locked: boolean;
    is_pinned: boolean;
    created_at: string;
}

export interface TopicsAnalysis {
    total_topics: number;
    total_comments: number;
    new_topics: number;
    new_comments: number;
}

async function getCategories(): Promise<Category[]> {
    return pull<Category[]>("/nodes");
}

// 获取帖子分析数据
async function getTopicsAnalysis(): Promise<TopicsAnalysis> {
    return pull<TopicsAnalysis>("/topics-analysis");
}

export interface PostSearchParams extends PaginationParams {
    query?: string; // 搜索标题或作者
    node_id?: number; // 分类ID
    per_page?: number;
}

// 获取帖子列表
async function getPosts(
    params: PostSearchParams = {}
): Promise<PaginationResponse<Post>> {
    const queryParams: Record<string, string> = {};

    if (params.p) queryParams.p = String(params.p);
    if (params.query) queryParams.query = params.query;
    if (params.node_id) queryParams.node_id = String(params.node_id);
    if (params.per_page) queryParams.per_page = String(params.per_page);


    return pull<PaginationResponse<Post>>(`/topics?${new URLSearchParams(queryParams).toString()}`);
}

// 更新帖子
async function updatePost(
    id: number,
    action: "pin" | "unpin" | "lock" | "unlock"
): Promise<void> {
    return push(`/topics/${id}`, {action});
}

// 删除帖子
async function deletePost(id: number): Promise<void> {
    return push(`/topics/${id}/delete`);
}

const TopicPage: React.FC = () => {
    const [currentPage, setCurrentPage] = useState(1);
    const [itemsPerPage, setItemsPerPage] = useState(20);
    const [searchQuery, setSearchQuery] = useState("");
    const [categoryFilter, setCategoryFilter] = useState("all");
    const [posts, setPosts] = useState<Post[]>([]);
    const [categories, setCategories] = useState<Category[]>([]);
    const [analysis, setAnalysis] = useState<TopicsAnalysis | null>(null);
    const [total, setTotal] = useState(0);
    const [totalPages, setTotalPages] = useState(0);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // 加载分类列表
    const loadCategories = async () => {
        try {
            const data = await getCategories();
            setCategories(data);
        } catch (err) {
            console.error("Failed to load categories:", err);
        }
    };

    // 加载分析数据
    const loadAnalysis = async () => {
        try {
            const data = await getTopicsAnalysis();
            setAnalysis(data);
        } catch (err) {
            console.error("Failed to load analysis:", err);
        }
    };

    // 加载帖子列表
    const loadPosts = async () => {
        setLoading(true);
        setError(null);
        try {
            const params: PostSearchParams = {
                p: currentPage,
                per_page: itemsPerPage
            };

            if (searchQuery) params.query = searchQuery;
            if (categoryFilter !== "all") {
                params.node_id = Number(categoryFilter);
            }

            const response = await getPosts(params);
            setPosts(response.data);
            setTotal(response.total);
            setTotalPages(response.total_page);
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "加载帖子列表失败";
            setError(errorMessage);
            console.error("Failed to load posts:", err);
        } finally {
            setLoading(false);
        }
    };

    // 初始加载分类和分析数据
    useEffect(() => {
        loadCategories();
        loadAnalysis();
    }, []);

    // 加载帖子
    useEffect(() => {
        loadPosts();
    }, [currentPage, searchQuery, categoryFilter, itemsPerPage]);

    // 筛选条件变化时回到第一页
    useEffect(() => {
        setCurrentPage(1);
    }, [searchQuery, categoryFilter, itemsPerPage]);

    const handleDelete = async (postId: number) => {
        if (!confirm("确定要删除这个帖子吗？")) return;

        try {
            await deletePost(postId);
            await loadPosts();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "删除帖子失败";
            alert(errorMessage);
        }
    };

    const handleUpdate = async (
        postId: number,
        action: "pin" | "unpin" | "lock" | "unlock"
    ) => {
        try {
            await updatePost(postId, action);
            await loadPosts();
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "更新帖子失败";
            alert(errorMessage);
        }
    };

    const getCategoryBadge = (nodeName: string) => {
        return <Badge color="blue">{nodeName}</Badge>;
    };

    return (
        <Container size="4">
            <Flex direction="column" gap="5">
                {/* Header */}
                <Box>
                    <Heading as="h1" size="6" mb="2">
                        帖子管理
                    </Heading>
                    <Text as="p" size="3" color="gray">
                        审核和管理论坛帖子内容
                    </Text>
                </Box>

                {/* Statistics Cards */}
                <Grid columns="4" gap="4">
                    <Card>
                        <Flex direction="column" gap="2">
                            <Text size="2" color="gray">
                                总帖子数
                            </Text>
                            <Text size="5" weight="bold" style={{color: "var(--blue-11)"}}>
                                {analysis?.total_topics ?? 0}
                            </Text>
                        </Flex>
                    </Card>
                    <Card>
                        <Flex direction="column" gap="2">
                            <Text size="2" color="gray">
                                总评论数
                            </Text>
                            <Text size="5" weight="bold" style={{color: "var(--green-11)"}}>
                                {analysis?.total_comments ?? 0}
                            </Text>
                        </Flex>
                    </Card>
                    <Card>
                        <Flex direction="column" gap="2">
                            <Text size="2" color="gray">
                                今日新帖
                            </Text>
                            <Text size="5" weight="bold" style={{color: "var(--orange-11)"}}>
                                {analysis?.new_topics ?? 0}
                            </Text>
                        </Flex>
                    </Card>
                    <Card>
                        <Flex direction="column" gap="2">
                            <Text size="2" color="gray">
                                今日评论
                            </Text>
                            <Text size="5" weight="bold" style={{color: "var(--red-11)"}}>
                                {analysis?.new_comments ?? 0}
                            </Text>
                        </Flex>
                    </Card>
                </Grid>

                {/* Filters */}
                <Card>
                    <Flex gap="3" wrap="wrap" align="end">
                        <Box style={{flex: "1", minWidth: "200px"}}>
                            <Text as="label" size="2" weight="medium" mb="1">
                                搜索
                            </Text>
                            <TextField.Root
                                placeholder="搜索帖子标题或作者..."
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                size="2"
                            />
                        </Box>

                        <Box style={{minWidth: "150px"}}>
                            <Text as="label" size="2" weight="medium" mb="1">
                                分类
                            </Text>
                            <Select.Root
                                value={categoryFilter}
                                onValueChange={setCategoryFilter}
                            >
                                <Select.Trigger style={{width: "100%"}}/>
                                <Select.Content>
                                    <Select.Item value="all">全部分类</Select.Item>
                                    {categories.map((cat) => (
                                        <Select.Item key={cat.id} value={String(cat.id)}>
                                            {cat.name}
                                        </Select.Item>
                                    ))}
                                </Select.Content>
                            </Select.Root>
                        </Box>

                        <Button
                            variant="soft"
                            onClick={() => {
                                setSearchQuery("");
                                setCategoryFilter("all");
                            }}
                        >
                            重置筛选
                        </Button>
                    </Flex>
                </Card>

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
                    找到 {total} 个帖子{loading && " (加载中...)"}
                </Text>

                {/* Posts Table */}
                <Card variant="ghost">
                    <Table.Root variant="surface">
                        <Table.Header>
                            <Table.Row>
                                <Table.ColumnHeaderCell>ID</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell style={{minWidth: "300px"}}>标题</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell style={{minWidth: "100px"}}>作者</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell style={{minWidth: "100px"}}>分类</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell style={{minWidth: "100px"}}>浏览</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell style={{minWidth: "120px"}}>创建时间</Table.ColumnHeaderCell>
                                <Table.ColumnHeaderCell style={{minWidth: "100px"}}>操作</Table.ColumnHeaderCell>
                            </Table.Row>
                        </Table.Header>

                        <Table.Body>
                            {posts.map((post) => (
                                <Table.Row key={post.id}>
                                    <Table.Cell>{post.id}</Table.Cell>
                                    <Table.Cell>
                                        <Text
                                            weight="medium"
                                            style={{maxWidth: "300px", display: "block"}}
                                        >
                                            {post.title}
                                        </Text>
                                    </Table.Cell>
                                    <Table.Cell>{post.username}</Table.Cell>
                                    <Table.Cell>{getCategoryBadge(post.node_name)}</Table.Cell>
                                    <Table.Cell>
                                        <Text size="2" color="gray">
                                            {post.view_count}
                                        </Text>
                                    </Table.Cell>
                                    <Table.Cell>
                                        {new Date(post.created_at).toLocaleDateString("zh-CN")}
                                    </Table.Cell>
                                    <Table.Cell>
                                        <Flex gap="2">
                                            <Button
                                                size="1"
                                                variant="soft"
                                                color={post.is_pinned ? "orange" : "gray"}
                                                onClick={() =>
                                                    handleUpdate(post.id, post.is_pinned ? "unpin" : "pin")
                                                }
                                                disabled={loading}
                                            >
                                                {post.is_pinned ? "取消置顶" : "置顶"}
                                            </Button>
                                            <Button
                                                size="1"
                                                variant="soft"
                                                color={post.is_locked ? "orange" : "gray"}
                                                onClick={() =>
                                                    handleUpdate(post.id, post.is_locked ? "unlock" : "lock")
                                                }
                                                disabled={loading}
                                            >
                                                {post.is_locked ? "解锁" : "锁定"}
                                            </Button>
                                            <Button
                                                size="1"
                                                variant="soft"
                                                color="red"
                                                onClick={() => handleDelete(post.id)}
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

                    {posts.length === 0 && !loading && (
                        <Card>
                            <Box p="4" style={{textAlign: "center"}}>
                                <Text color="gray">暂无数据</Text>
                            </Box>
                        </Card>
                    )}
                </Card>

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
        </Container>
    );
};

export default TopicPage;
