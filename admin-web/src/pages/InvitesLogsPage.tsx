import React, {useEffect, useState} from "react";
import {Button, Dialog, Flex, Table} from "@radix-ui/themes";
import {pull} from "../api";

interface InvitesLogsPageProps {
    inviteId: number | null;
    open: boolean;
    onOpenChange: (open: boolean) => void;
}

export interface InviteUsageLog {
    id: number;
    invitation_id: number;
    user_id: number;
    username: string;
    used_at: string;
}

// 获取邀请码使用记录
async function getInviteLogs(id: number): Promise<InviteUsageLog[]> {
    return pull<InviteUsageLog[]>(`/invites/${id}/logs`);
}

const InvitesLogsPage: React.FC<InvitesLogsPageProps> = ({inviteId, open, onOpenChange}) => {
    const [logs, setLogs] = useState<InviteUsageLog[]>([]);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
        if (open && inviteId) {
            loadLogs(inviteId);
        } else {
            setLogs([]);
        }
    }, [open, inviteId]);

    const loadLogs = async (id: number) => {
        setLoading(true);
        try {
            const data = await getInviteLogs(id);
            setLogs(data);
        } catch (err) {
            console.error(err);
        } finally {
            setLoading(false);
        }
    };

    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Content style={{maxWidth: 600}}>
                <Dialog.Title>使用记录</Dialog.Title>
                <Table.Root>
                    <Table.Header>
                        <Table.Row>
                            <Table.ColumnHeaderCell>用户</Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell>使用时间</Table.ColumnHeaderCell>
                        </Table.Row>
                    </Table.Header>
                    <Table.Body>
                        {loading ? (
                            <Table.Row>
                                <Table.Cell colSpan={2}>加载中...</Table.Cell>
                            </Table.Row>
                        ) : logs.length > 0 ? (
                            logs.map((log) => (
                                <Table.Row key={log.id}>
                                    <Table.Cell>{log.username} (ID: {log.user_id})</Table.Cell>
                                    <Table.Cell>{new Date(log.used_at).toLocaleString()}</Table.Cell>
                                </Table.Row>
                            ))
                        ) : (
                            <Table.Row>
                                <Table.Cell colSpan={2}>暂无记录</Table.Cell>
                            </Table.Row>
                        )}
                    </Table.Body>
                </Table.Root>
                <Flex justify="end" mt="4">
                    <Dialog.Close>
                        <Button variant="soft">关闭</Button>
                    </Dialog.Close>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};

export default InvitesLogsPage;
