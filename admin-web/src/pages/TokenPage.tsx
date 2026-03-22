import React, {useEffect, useState} from "react";
import {Box, Button, Card, Dialog, Flex, Heading, IconButton, Table, Text, TextField,} from "@radix-ui/themes";
import {CopyIcon, PlusIcon, TrashIcon} from "@radix-ui/react-icons";
import {pull, push} from "../api";

export interface AccessToken {
    token: string;
    description: string;
    created_at: string;
    expires_at: string;
    user_id: number;
    category: string;
    username: string;
}

export interface CreateTokenParams {
    description: string;
    user_id?: number;
}

// 获取 Token 列表
async function getTokens(): Promise<AccessToken[]> {
    return pull<AccessToken[]>("/tokens");
}

// 创建 Token
async function createToken(params: CreateTokenParams): Promise<string> {
    return push<string, CreateTokenParams>("/tokens", params);
}

// 删除 Token
async function deleteToken(token: string): Promise<void> {
    return push(`/tokens/${token}/delete`);
}

const TokenPage: React.FC = () => {
    const [tokens, setTokens] = useState<AccessToken[]>([]);
    const [loading, setLoading] = useState(true);
    const [createOpen, setCreateOpen] = useState(false);
    const [description, setDescription] = useState("");
    const [userId, setUserId] = useState("");
    const [createdToken, setCreatedToken] = useState<string | null>(null);

    const fetchTokens = async () => {
        try {
            setLoading(true);
            const data = await getTokens();
            setTokens(data);
        } catch (error) {
            console.error("Failed to fetch tokens", error);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchTokens();
    }, []);

    const handleCreate = async () => {
        if (!description) return;
        try {
            const uid = userId ? parseInt(userId) : undefined;
            const token = await createToken({description, user_id: uid});
            setCreatedToken(token);
            setDescription("");
            setUserId("");
            fetchTokens();
        } catch (error) {
            console.error("Failed to create token", error);
            alert("创建失败，请检查用户 ID 是否存在");
        }
    };

    const handleDelete = async (token: string) => {
        if (!confirm("确定要删除这个 Token 吗？")) return;
        try {
            await deleteToken(token);
            fetchTokens();
        } catch (error) {
            console.error("Failed to delete token", error);
            alert("删除失败");
        }
    };

    const copyToClipboard = (text: string) => {
        navigator.clipboard.writeText(text);
        alert("Token 已复制到剪贴板");
    };

    return (
        <Box>
            <Flex justify="between" align="center" mb="4">
                <Heading size="6">机器人令牌管理</Heading>
                <Dialog.Root open={createOpen} onOpenChange={setCreateOpen}>
                    <Dialog.Trigger>
                        <Button>
                            <PlusIcon/> 创建令牌
                        </Button>
                    </Dialog.Trigger>
                    <Dialog.Content style={{maxWidth: 450}}>
                        <Dialog.Title>创建机器人令牌</Dialog.Title>
                        <Dialog.Description size="2" mb="4">
                            创建一个新的 API 访问令牌，用于机器人程序。
                        </Dialog.Description>

                        {createdToken ? (
                            <Flex direction="column" gap="3">
                                <Text color="green" weight="bold">创建成功！请立即复制保存，之后无法再次查看。</Text>
                                <Card>
                                    <Flex justify="between" align="center">
                                        <Text style={{wordBreak: "break-all"}}>{createdToken}</Text>
                                        <IconButton variant="ghost" onClick={() => copyToClipboard(createdToken)}>
                                            <CopyIcon/>
                                        </IconButton>
                                    </Flex>
                                </Card>
                                <Flex gap="3" mt="4" justify="end">
                                    <Dialog.Close>
                                        <Button onClick={() => {
                                            setCreatedToken(null);
                                            setCreateOpen(false);
                                        }}>关闭</Button>
                                    </Dialog.Close>
                                </Flex>
                            </Flex>
                        ) : (
                            <Flex direction="column" gap="3">
                                <label>
                                    <Text as="div" size="2" mb="1" weight="bold">
                                        描述
                                    </Text>
                                    <TextField.Root
                                        placeholder="例如：自动回复机器人"
                                        value={description}
                                        onChange={(e) => setDescription(e.target.value)}
                                    />
                                </label>

                                <label>
                                    <Text as="div" size="2" mb="1" weight="bold">
                                        用户 ID (可选)
                                    </Text>
                                    <TextField.Root
                                        type="number"
                                        placeholder="留空则默认为当前管理员"
                                        value={userId}
                                        onChange={(e) => setUserId(e.target.value)}
                                    />
                                    <Text size="1" color="gray">
                                        如果不指定，令牌将绑定到当前管理员账号。建议为机器人创建单独的用户账号。
                                    </Text>
                                </label>

                                <Flex gap="3" mt="4" justify="end">
                                    <Dialog.Close>
                                        <Button variant="soft" color="gray">
                                            取消
                                        </Button>
                                    </Dialog.Close>
                                    <Button onClick={handleCreate}>创建</Button>
                                </Flex>
                            </Flex>
                        )}
                    </Dialog.Content>
                </Dialog.Root>
            </Flex>

            <Card variant="ghost">
                <Table.Root variant="surface">
                    <Table.Header>
                        <Table.Row>
                            <Table.ColumnHeaderCell style={{minWidth: "150px"}}>描述</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>Token (前缀)</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>创建时间</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "120px"}}>过期时间</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "100px"}}>创建者</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{minWidth: "100px"}}>操作</Table.ColumnHeaderCell>
                        </Table.Row>
                    </Table.Header>

                    <Table.Body>
                        {loading ? (
                            <Table.Row>
                                <Table.Cell colSpan={6} style={{textAlign: "center"}}>
                                    加载中...
                                </Table.Cell>
                            </Table.Row>
                        ) : tokens.length === 0 ? (
                            <Table.Row>
                                <Table.Cell colSpan={6} style={{textAlign: "center"}}>
                                    暂无数据
                                </Table.Cell>
                            </Table.Row>
                        ) : (
                            tokens.map((token) => (
                                <Table.Row key={token.token}>
                                    <Table.Cell>{token.description}</Table.Cell>
                                    <Table.Cell>{token.token.substring(0, 8)}...</Table.Cell>
                                    <Table.Cell>{new Date(token.created_at).toLocaleString()}</Table.Cell>
                                    <Table.Cell>{new Date(token.expires_at).toLocaleString()}</Table.Cell>
                                    <Table.Cell>{token.username}</Table.Cell>
                                    <Table.Cell>
                                        <Flex gap="2">
                                            <IconButton
                                                color="red"
                                                variant="soft"
                                                onClick={() => handleDelete(token.token)}
                                            >
                                                <TrashIcon/>
                                            </IconButton>
                                        </Flex>
                                    </Table.Cell>
                                </Table.Row>
                            ))
                        )}
                    </Table.Body>
                </Table.Root>
            </Card>
        </Box>
    );
};

export default TokenPage;
