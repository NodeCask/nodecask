import React, { useEffect, useReducer, useState } from "react";
import { Box, Button, Card, Flex, Heading, Separator, Spinner, Switch, Text, TextField, } from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";

// 发送测试邮件
async function sendTestEmail(params: {
    email: string;
    title: string;
    content: string;
}): Promise<void> {
    return push("/email-test", params);
}

export interface SmtpConfig {
    from_name: string;
    from_mail: string;
    hostname: string;
    port: number;
    tls_implicit: boolean;
    username: string;
    password: string;
    max_per_hour: number;
    max_per_day: number;
}

const load = () => pull<SmtpConfig>(`/settings?name=smtp_config`);
const save = (value: SmtpConfig) => push(`/settings`, { name: "smtp_config", value });

export const SmtpSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [testMessage, setTestMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [form, setForm] = useState<SmtpConfig>({
        from_mail: "",
        from_name: "",
        hostname: "",
        password: "",
        port: 0,
        tls_implicit: false,
        username: "",
        max_per_hour: 0,
        max_per_day: 0
    });
    const [testEmail, setTestEmail] = useState("");
    const [sendingTest, setSendingTest] = useState(false);

    useEffect(() => {
        load()
            .then(v => v && setForm(v))
            .finally(() => dispatch({ type: "STOP_LOADING" }));
    }, []);

    if (spinner.loading) {
        return (
            <Card>
                <Flex align="center" justify="center" py="9">
                    <Spinner size="3" />
                </Flex>
            </Card>
        );
    }

    const handleSave = async () => {
        dispatch({ type: "START_SAVING" });
        try {
            await save(form);
            setMessage({ text: "邮件设置保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存邮件设置失败";
            setMessage({ text: errorMessage, type: "error" });
        } finally {
            dispatch({ type: "STOP_SAVING" });
            setTimeout(() => setMessage(null), 3000);
        }
    };

    const handleSendTest = async () => {
        if (!testEmail) {
            setTestMessage({ text: "请输入收件人邮箱", type: "error" });
            setTimeout(() => setTestMessage(null), 3000);
            return;
        }
        setSendingTest(true);
        try {
            await sendTestEmail({
                email: testEmail,
                title: "SMTP Test",
                content: "If you see this message, it indicates that the SMTP configuration is correct.",
            });
            setTestMessage({ text: "测试邮件请求已发送", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "发送测试邮件失败";
            setTestMessage({ text: errorMessage, type: "error" });
        } finally {
            setSendingTest(false);
            setTimeout(() => setTestMessage(null), 3000);
        }
    };

    return (
        <Card>
            <Flex direction="column" gap="4">
                <Box>
                    <Heading as="h3" size="4" mb="3">
                        SMTP 配置
                    </Heading>
                    <Flex direction="column" gap="3">
                        <Flex gap="3">
                            <Box style={{ flex: 1 }}>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    发件人名称
                                </Text>
                                <TextField.Root
                                    value={form.from_name}
                                    onChange={(e) => setForm({ ...form, from_name: e.target.value })}
                                    placeholder="Tiny BBS"
                                />
                            </Box>
                            <Box style={{ flex: 1 }}>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    发件人邮箱
                                </Text>
                                <TextField.Root
                                    value={form.from_mail}
                                    onChange={(e) => setForm({ ...form, from_mail: e.target.value })}
                                    placeholder="noreply@example.com"
                                />
                            </Box>
                        </Flex>

                        <Flex gap="3">
                            <Box style={{ flex: 1 }}>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    SMTP 服务器
                                </Text>
                                <TextField.Root
                                    value={form.hostname}
                                    onChange={(e) => setForm({ ...form, hostname: e.target.value })}
                                    placeholder="smtp.example.com"
                                />
                            </Box>
                            <Box style={{ width: "120px" }}>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    SMTP 端口
                                </Text>
                                <TextField.Root
                                    type="number"
                                    value={String(form.port)}
                                    onChange={(e) =>
                                        setForm({ ...form, port: Number(e.target.value) })
                                    }
                                    placeholder="465"
                                />
                            </Box>
                        </Flex>

                        <Box>
                            <Flex align="center" gap="2">
                                <Switch
                                    checked={form.tls_implicit}
                                    onCheckedChange={(checked) =>
                                        setForm({ ...form, tls_implicit: checked })
                                    }
                                />
                                <Text size="2" weight="medium">
                                    使用隐式 TLS (Implicit TLS)
                                </Text>
                            </Flex>
                            <Text size="1" color="gray" mt="1">
                                通常 465 端口使用隐式 TLS，587 端口使用显式 TLS (STARTTLS)。
                            </Text>
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                SMTP 用户名
                            </Text>
                            <TextField.Root
                                value={form.username}
                                onChange={(e) => setForm({ ...form, username: e.target.value })}
                                placeholder="admin@example.com"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                SMTP 密码
                            </Text>
                            <TextField.Root
                                type="password"
                                value={form.password}
                                onChange={(e) =>
                                    setForm({ ...form, password: e.target.value })
                                }
                                placeholder="输入 SMTP 密码"
                            />
                        </Box>

                        <Flex gap="3">
                            <Box style={{ flex: 1 }}>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    每小时发送限制 (0为不限制)
                                </Text>
                                <TextField.Root
                                    type="number"
                                    value={String(form.max_per_hour)}
                                    onChange={(e) =>
                                        setForm({ ...form, max_per_hour: Number(e.target.value) })
                                    }
                                    placeholder="0"
                                />
                            </Box>
                            <Box style={{ flex: 1 }}>
                                <Text as="label" size="2" weight="medium" mb="1">
                                    每天发送限制 (0为不限制)
                                </Text>
                                <TextField.Root
                                    type="number"
                                    value={String(form.max_per_day)}
                                    onChange={(e) =>
                                        setForm({ ...form, max_per_day: Number(e.target.value) })
                                    }
                                    placeholder="0"
                                />
                            </Box>
                        </Flex>
                    </Flex>
                </Box>

                <Separator size="4" />

                <Flex justify="end" align="center" gap="3">
                    {message && (
                        <Text size="2" color={message.type === "success" ? "green" : "red"}>
                            {message.text}
                        </Text>
                    )}
                    <Button onClick={handleSave} loading={spinner.saving} disabled={spinner.saving}>保存邮件设置</Button>
                </Flex>

                <Separator size="4" />

                <Box>
                    <Heading as="h3" size="4" mb="3">
                        邮件发送测试
                    </Heading>
                    <Flex gap="3" align="end">
                        <Box style={{ flex: 1 }}>
                            <Text as="label" size="2" weight="medium" mb="1">
                                收件人邮箱
                            </Text>
                            <TextField.Root
                                value={testEmail}
                                onChange={(e) => setTestEmail(e.target.value)}
                                placeholder="test@example.com"
                            />
                        </Box>
                        {testMessage && (
                            <Text size="2" color={testMessage.type === "success" ? "green" : "red"}>
                                {testMessage.text}
                            </Text>
                        )}
                        <Button
                            variant="surface"
                            onClick={handleSendTest}
                            disabled={sendingTest}
                            loading={sendingTest}
                        >
                            发送测试邮件
                        </Button>
                    </Flex>
                    <Text size="1" color="gray" mt="2">
                        注意：发送测试邮件前请确保已保存上方的配置。
                    </Text>
                </Box>
            </Flex>
        </Card>
    );
};
