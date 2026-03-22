import React, {useEffect, useState} from "react";
import {
    Box,
    Button,
    Card,
    Container,
    Dialog,
    Flex,
    Heading,
    IconButton,
    Table,
    Text,
    TextField,
} from "@radix-ui/themes";
import {CopyIcon, InfoCircledIcon, TrashIcon} from "@radix-ui/react-icons";
import Pagination from "../components/Pagination";
import InvitesLogsPage from "./InvitesLogsPage";
import {type PaginationParams, type PaginationResponse, pull, push, SysError} from "../api";

export interface InviteCode {
    id: number;
    code: string;
    quota: number;
    used_count: number;
    created_at: string;
    expired_at: string | null;
}

export interface InviteGenerateParams {
    count: number;
    quota: number;
    expired_at?: string;
}

export type InviteSearchParams = PaginationParams

// 获取邀请码列表
async function getInvites(
    params: InviteSearchParams = {}
): Promise<PaginationResponse<InviteCode>> {
    const queryParams: Record<string, string> = {};
    if (params.p) queryParams.p = String(params.p);
    return pull<PaginationResponse<InviteCode>>(`/invites?${new URLSearchParams(queryParams).toString()}`,);
}

// 生成邀请码
async function generateInvites(
    params: InviteGenerateParams
): Promise<string[]> {
    return push<string[], InviteGenerateParams>("/invites/generate", params);
}

// 删除邀请码
async function deleteInvite(id: number): Promise<void> {
    return push(`/invites/${id}/delete`);
}


const InviteCodePage: React.FC = () => {
    const [currentPage, setCurrentPage] = useState(1);
    const [invites, setInvites] = useState<InviteCode[]>([]);
    const [total, setTotal] = useState(0);
    const [totalPages, setTotalPages] = useState(0);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Generate Dialog State
    const [generateDialogOpen, setGenerateDialogOpen] = useState(false);
    const [generateCount, setGenerateCount] = useState(1);
    const [generateQuota, setGenerateQuota] = useState(1);
    const [generateExpiredAt, setGenerateExpiredAt] = useState("");

    // Logs Dialog State
    const [logsDialogOpen, setLogsDialogOpen] = useState(false);
    const [selectedInviteId, setSelectedInviteId] = useState<number | null>(null);

    // Load Invites
    const loadInvites = async () => {
        setLoading(true);
        setError(null);
        try {
            const response = await getInvites({p: currentPage});
            setInvites(response.data);
            setTotal(response.total);
            setTotalPages(response.total_page);
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "加载邀请码失败";
            setError(errorMessage);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadInvites();
    }, [currentPage]);

    // Handle Generate
    const handleGenerate = async () => {
        try {
            await generateInvites({
                count: generateCount,
                quota: generateQuota,
                expired_at: generateExpiredAt ? new Date(generateExpiredAt).toISOString() : undefined
            });
            setGenerateDialogOpen(false);
            setGenerateCount(1);
            setGenerateQuota(1);
            setGenerateExpiredAt("");
            loadInvites();
        } catch (err) {
            alert(err instanceof SysError ? err.message : "生成失败");
        }
    };

    // Handle Delete
    const handleDelete = async (id: number) => {
        if (!confirm("确定要删除这个邀请码吗？")) return;
        try {
            await deleteInvite(id);
            loadInvites();
        } catch (err) {
            alert(err instanceof SysError ? err.message : "删除失败");
        }
    };

    // Handle View Logs
    const handleViewLogs = (id: number) => {
        setSelectedInviteId(id);
        setLogsDialogOpen(true);
    };

    const copyToClipboard = (text: string) => {
        navigator.clipboard.writeText(text);
        // Could add a toast here
    };

    return (
        <Container size="4">
            <Flex direction="column" gap="5">
                <Flex justify="between" align="center">
                    <Box>
                        <Heading as="h1" size="6" mb="2">
                            邀请码管理
                        </Heading>
                        <Text as="p" size="3" color="gray">
                            生成和管理注册邀请码
                        </Text>
                    </Box>
                    <Dialog.Root open={generateDialogOpen} onOpenChange={setGenerateDialogOpen}>
                        <Dialog.Trigger>
                            <Button>生成邀请码</Button>
                        </Dialog.Trigger>
                        <Dialog.Content style={{maxWidth: 450}}>
                            <Dialog.Title>生成邀请码</Dialog.Title>
                            <Flex direction="column" gap="3">
                                <Box>
                                    <Text as="label" size="2" mb="1" weight="bold">
                                        生成数量
                                    </Text>
                                    <TextField.Root
                                        type="number"
                                        min="1"
                                        max="100"
                                        value={generateCount}
                                        onChange={(e) => setGenerateCount(parseInt(e.target.value) || 1)}
                                    />
                                </Box>
                                <Box>
                                    <Text as="label" size="2" mb="1" weight="bold">
                                        使用额度 (每个码可使用次数)
                                    </Text>
                                    <TextField.Root
                                        type="number"
                                        min="1"
                                        value={generateQuota}
                                        onChange={(e) => setGenerateQuota(parseInt(e.target.value) || 1)}
                                    />
                                </Box>
                                <Box>
                                    <Text as="label" size="2" mb="1" weight="bold">
                                        过期时间 (可选)
                                    </Text>
                                    <TextField.Root
                                        type="datetime-local"
                                        value={generateExpiredAt}
                                        onChange={(e) => setGenerateExpiredAt(e.target.value)}
                                    />
                                </Box>
                            </Flex>
                            <Flex gap="3" mt="4" justify="end">
                                <Dialog.Close>
                                    <Button variant="soft" color="gray">
                                        取消
                                    </Button>
                                </Dialog.Close>
                                <Button onClick={handleGenerate}>确认生成</Button>
                            </Flex>
                        </Dialog.Content>
                    </Dialog.Root>
                </Flex>

                {error && (
                    <Card>
                        <Text color="red">{error}</Text>
                    </Card>
                )}

                <Table.Root variant="surface">
                    <Table.Header>
                        <Table.Row>
                            <Table.ColumnHeaderCell>ID</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "350px"}}>邀请码</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "180px"}}>进度 (已用/额度)</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>创建时间</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>过期时间</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "100px"}}>操作</Table.ColumnHeaderCell>
                        </Table.Row>
                    </Table.Header>
                    <Table.Body>
                        {invites.map((invite) => (
                            <Table.Row key={invite.id}>
                                <Table.Cell>{invite.id}</Table.Cell>
                                <Table.Cell>
                                    <Flex align="center" gap="2">
                                        <Text style={{fontFamily: "monospace"}}>{invite.code}</Text>
                                        <IconButton
                                            size="1"
                                            variant="ghost"
                                            onClick={() => copyToClipboard(invite.code)}
                                            title="复制"
                                        >
                                            <CopyIcon/>
                                        </IconButton>
                                    </Flex>
                                </Table.Cell>
                                <Table.Cell>
                                    <Text>
                                        {invite.used_count} / {invite.quota}
                                    </Text>
                                    {invite.used_count > 0 &&
                                        <IconButton
                                            size="1"
                                            variant="soft"
                                            onClick={() => handleViewLogs(invite.id)}
                                            title="查看使用记录"
                                            disabled={invite.used_count === 0}
                                        >
                                            <InfoCircledIcon/>
                                        </IconButton>}
                                </Table.Cell>
                                <Table.Cell>
                                    {new Date(invite.created_at).toLocaleString()}
                                </Table.Cell>
                                <Table.Cell>
                                    {invite.expired_at ? new Date(invite.expired_at).toLocaleString() : "-"}
                                </Table.Cell>
                                <Table.Cell>
                                    <Flex gap="2">
                                        <IconButton
                                            color="red"
                                            variant="soft"
                                            onClick={() => handleDelete(invite.id)}
                                            title="删除"
                                        >
                                            <TrashIcon/>
                                        </IconButton>
                                    </Flex>
                                </Table.Cell>
                            </Table.Row>
                        ))}
                    </Table.Body>
                </Table.Root>
                {invites.length === 0 && !loading && (
                    <Card>
                        <Box p="4" style={{textAlign: "center"}}>
                            <Text color="gray">暂无数据</Text>
                        </Box>
                    </Card>
                )}

                {totalPages > 1 && (
                    <Pagination
                        currentPage={currentPage}
                        totalPages={totalPages}
                        onPageChange={setCurrentPage}
                        itemsPerPage={20}
                        onItemsPerPageChange={() => {
                        }}
                    />
                )}
            </Flex>

            {/* Logs Dialog */}
            <InvitesLogsPage
                inviteId={selectedInviteId}
                open={logsDialogOpen}
                onOpenChange={setLogsDialogOpen}
            />
        </Container>
    );
};

export default InviteCodePage;