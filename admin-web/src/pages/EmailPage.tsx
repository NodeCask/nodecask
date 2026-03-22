import React, {useEffect, useState} from "react";
import {Badge, Box, Button, Card, Container, Dialog, Flex, Heading, ScrollArea, Table, Tabs, Text,} from "@radix-ui/themes";
import Pagination from "../components/Pagination";
import {type PaginationParams, type PaginationResponse, pull, SysError} from "../api";

export interface Email {
    id: number;
    created_at: string;
    user_id: number;
    email_from: string;
    email_to: string;
    email_subject: string;
    result: string;
}

export interface EmailDetail extends Email {
    email_body: string;
}

export interface EmailSearchParams extends PaginationParams {
    per_page?: number;
}

// 获取邮件列表
async function getEmails(
    params: EmailSearchParams = {}
): Promise<PaginationResponse<Email>> {
    const queryParams: Record<string, string> = {};

    if (params.p) queryParams.p = String(params.p);
    if (params.per_page) queryParams.per_page = String(params.per_page);

    return pull<PaginationResponse<Email>>(`/emails?${new URLSearchParams(queryParams).toString()}`);
}

// 获取邮件详情
async function getEmail(id: number): Promise<EmailDetail> {
    return pull<EmailDetail>(`/emails/${id}`);
}


const EmailPage: React.FC = () => {
    const [currentPage, setCurrentPage] = useState(1);
    const [itemsPerPage, setItemsPerPage] = useState(20);
    const [emails, setEmails] = useState<Email[]>([]);
    const [total, setTotal] = useState(0);
    const [totalPages, setTotalPages] = useState(0);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // 详情对话框状态
    const [selectedEmailId, setSelectedEmailId] = useState<number | null>(null);
    const [emailDetail, setEmailDetail] = useState<EmailDetail | null>(null);
    const [detailLoading, setDetailLoading] = useState(false);

    // 加载邮件列表
    const loadEmails = async () => {
        setLoading(true);
        setError(null);
        try {
            const params = {
                p: currentPage,
                per_page: itemsPerPage
            };

            const response = await getEmails(params);
            setEmails(response.data);
            setTotal(response.total);
            setTotalPages(response.total_page);
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "加载邮件列表失败";
            setError(errorMessage);
            console.error("Failed to load emails:", err);
        } finally {
            setLoading(false);
        }
    };

    // 加载邮件详情
    const loadEmailDetail = async (id: number) => {
        setDetailLoading(true);
        try {
            const detail = await getEmail(id);
            setEmailDetail(detail);
        } catch (err) {
            console.error("Failed to load email detail:", err);
        } finally {
            setDetailLoading(false);
        }
    };

    // 页面加载和翻页时重新加载
    useEffect(() => {
        loadEmails();
    }, [currentPage, itemsPerPage]);

    // 重置页面
    useEffect(() => {
        setCurrentPage(1);
    }, [itemsPerPage]);

    // 当选中的邮件ID变化时加载详情
    useEffect(() => {
        if (selectedEmailId) {
            loadEmailDetail(selectedEmailId);
        } else {
            setEmailDetail(null);
        }
    }, [selectedEmailId]);

    const getResultBadge = (result: string) => {
        if (result === "pending") {
            return <Badge color="orange">等待发送</Badge>;
        } else if (result === "sent") {
            return <Badge color="green">发送成功</Badge>;
        } else if (result === "drop") {
            return <Badge color="red">丢弃</Badge>;
        } else if (result === "limited") {
            return <Badge color="red">超过限额</Badge>;
        } else if (result === "failed") {
            return <Badge color="red">发送失败</Badge>;
        } else {
            return <Badge color="red">{result}</Badge>;
        }
    };

    return (
        <Container size="4">
            <Flex direction="column" gap="5">
                {/* Header */}
                <Box>
                    <Heading as="h1" size="6" mb="2">
                        邮件队列
                    </Heading>
                    <Text as="p" size="3" color="gray">
                        查看系统发送的邮件记录
                    </Text>
                </Box>

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
                    找到 {total} 封邮件{loading && " (加载中...)"}
                </Text>

                {/* Emails Table */}
                <Table.Root variant="surface">
                    <Table.Header>
                        <Table.Row>
                            <Table.ColumnHeaderCell>ID</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>创建时间</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "150px"}}>接收者</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "200px"}}>主题</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "100px"}}>状态</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "100px"}}>操作</Table.ColumnHeaderCell>
                        </Table.Row>
                    </Table.Header>

                    <Table.Body>
                        {emails.map((email) => (
                            <Table.Row key={email.id}>
                                <Table.Cell>{email.id}</Table.Cell>
                                <Table.Cell>
                                    {new Date(email.created_at).toLocaleString("zh-CN")}
                                </Table.Cell>
                                <Table.Cell>{email.email_to}</Table.Cell>
                                <Table.Cell>{email.email_subject}</Table.Cell>
                                <Table.Cell>{getResultBadge(email.result)}</Table.Cell>
                                <Table.Cell>
                                    <Dialog.Root
                                        onOpenChange={(open) => {
                                            if (!open) setSelectedEmailId(null);
                                        }}
                                    >
                                        <Dialog.Trigger>
                                            <Button
                                                size="1"
                                                variant="soft"
                                                onClick={() => setSelectedEmailId(email.id)}
                                            >
                                                查看详情
                                            </Button>
                                        </Dialog.Trigger>

                                        <Dialog.Content style={{maxWidth: 600}}>
                                            <Dialog.Title>邮件详情 #{selectedEmailId}</Dialog.Title>

                                            {detailLoading ? (
                                                <Text>加载中...</Text>
                                            ) : emailDetail ? (
                                                <Flex direction="column" gap="3">
                                                    <Box>
                                                        <Text weight="bold">发送者: </Text>
                                                        <Text>{emailDetail.email_from}</Text>
                                                    </Box>
                                                    <Box>
                                                        <Text weight="bold">接收者: </Text>
                                                        <Text>{emailDetail.email_to}</Text>
                                                    </Box>
                                                    <Box>
                                                        <Text weight="bold">主题: </Text>
                                                        <Text>{emailDetail.email_subject}</Text>
                                                    </Box>
                                                    <Box>
                                                        <Text weight="bold">创建时间: </Text>
                                                        <Text>{new Date(emailDetail.created_at).toLocaleString("zh-CN")}</Text>
                                                    </Box>
                                                    <Box>
                                                        <Text weight="bold">发送状态: </Text>
                                                        <Text>{emailDetail.result}</Text>
                                                    </Box>

                                                    <Box>
                                                        <Text weight="bold" mb="1" as="div">邮件内容: </Text>
                                                        <Tabs.Root defaultValue="preview">
                                                            <Tabs.List>
                                                                <Tabs.Trigger value="preview">预览</Tabs.Trigger>
                                                                <Tabs.Trigger value="raw">原文</Tabs.Trigger>
                                                            </Tabs.List>

                                                            <Box pt="3">
                                                                <Tabs.Content value="preview">
                                                                    <Card style={{
                                                                        backgroundColor: "white",
                                                                        padding: 0,
                                                                        overflow: "hidden"
                                                                    }}>
                                                                        <iframe
                                                                            title="Email Content"
                                                                            srcDoc={emailDetail.email_body}
                                                                            style={{
                                                                                width: "100%",
                                                                                height: "400px",
                                                                                border: "none",
                                                                                display: "block"
                                                                            }}
                                                                            sandbox=""
                                                                        />
                                                                    </Card>
                                                                </Tabs.Content>

                                                                <Tabs.Content value="raw">
                                                                    <Card style={{backgroundColor: "var(--gray-2)"}}>
                                                                        <ScrollArea type="auto"
                                                                                    scrollbars="vertical"
                                                                                    style={{maxHeight: 400}}>
                                                                            <Box p="2">
                                                                                    <pre style={{
                                                                                        whiteSpace: "pre-wrap",
                                                                                        wordBreak: "break-word",
                                                                                        margin: 0
                                                                                    }}>
                                                                                        {emailDetail.email_body}
                                                                                    </pre>
                                                                            </Box>
                                                                        </ScrollArea>
                                                                    </Card>
                                                                </Tabs.Content>
                                                            </Box>
                                                        </Tabs.Root>
                                                    </Box>
                                                </Flex>
                                            ) : (
                                                <Text color="red">加载详情失败</Text>
                                            )}

                                            <Flex gap="3" mt="4" justify="end">
                                                <Dialog.Close>
                                                    <Button variant="soft" color="gray">
                                                        关闭
                                                    </Button>
                                                </Dialog.Close>
                                            </Flex>
                                        </Dialog.Content>
                                    </Dialog.Root>
                                </Table.Cell>
                            </Table.Row>
                        ))}
                    </Table.Body>
                </Table.Root>

                {emails.length === 0 && !loading && (
                    <Card>
                        <Box p="4" style={{textAlign: "center"}}>
                            <Text color="gray">暂无数据</Text>
                        </Box>
                    </Card>
                )}

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

export default EmailPage;
