import React, { useEffect, useReducer, useState } from "react";
import { Box, Button, Card, Flex, Heading, Separator, Spinner, Text, TextArea, } from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";

export interface LinkFilterConfig {
    rules: string;
}

const load = () => pull<LinkFilterConfig>(`/settings?name=link_filter`);
const save = (value: LinkFilterConfig) => push(`/settings`, { name: "link_filter", value });

export const LinkFilterSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [form, setForm] = useState<LinkFilterConfig>({
        rules: "",
    });

    useEffect(() => {
        load()
            .then((data) => {
                if (data) {
                    setForm(data);
                }
            })
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
            setMessage({ text: "链接过滤设置保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存链接过滤设置失败";
            setMessage({ text: errorMessage, type: "error" });
        } finally {
            dispatch({ type: "STOP_SAVING" });
            setTimeout(() => setMessage(null), 3000);
        }
    };

    return (
        <Card>
            <Flex direction="column" gap="4">
                <Box>
                    <Heading as="h3" size="4" mb="3">
                        链接过滤设置
                    </Heading>
                    <Box mb="4">
                        <Text as="p" size="2" color="gray" mb="2">
                            使用 AdBlock 兼容规则来过滤发帖和回复中的站外链接。
                        </Text>
                        <Card variant="surface">
                            <Box p="2">
                                <Heading as="h4" size="2" mb="2">规则编写说明：</Heading>
                                <Text as="div" size="1">
                                    <ul style={{ margin: 0, paddingLeft: '20px' }}>
                                        <li><code>||example.com^</code> - 屏蔽 example.com 及其所有子域名</li>
                                        <li><code>@@||example.com^</code> - 白名单，允许 example.com</li>
                                        <li><code>/badword/</code> - 使用正则表达式屏蔽包含特定词汇的链接</li>
                                        <li>每行一条规则</li>
                                    </ul>
                                </Text>
                            </Box>
                        </Card>
                    </Box>
                    <Flex direction="column" gap="3">
                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                过滤规则
                            </Text>
                            <TextArea
                                value={form.rules}
                                onChange={(e) =>
                                    setForm({ ...form, rules: e.target.value })
                                }
                                placeholder="||baidu.com^&#10;||google.com^"
                                rows={15}
                                style={{ fontFamily: "monospace" }}
                            />
                        </Box>
                    </Flex>
                </Box>

                <Separator size="4" />

                <Flex justify="end" align="center" gap="3">
                    {message && (
                        <Text size="2" color={message.type === "success" ? "green" : "red"}>
                            {message.text}
                        </Text>
                    )}
                    <Button onClick={handleSave} loading={spinner.saving} disabled={spinner.saving}>保存设置</Button>
                </Flex>
            </Flex>
        </Card>
    );
};
