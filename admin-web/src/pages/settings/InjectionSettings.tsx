import React, { useEffect, useReducer, useState } from "react";
import { Box, Button, Card, Flex, Heading, Separator, Spinner, Text, Tabs } from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";
import Editor from "@monaco-editor/react";

export interface InjectionConfig {
    html_head: string;
    html_body: string;
    style: string;
}

const load = () => pull<InjectionConfig>(`/settings?name=injection_config`);
const save = (value: InjectionConfig) => push(`/settings`, { name: "injection_config", value });

export const InjectionSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [form, setForm] = useState<InjectionConfig>({
        html_head: "",
        html_body: "",
        style: "",
    });

    useEffect(() => {
        load()
            .then((v) => v && setForm({
                html_head: v.html_head || "",
                html_body: v.html_body || "",
                style: v.style || ""
            }))
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
            setMessage({ text: "代码注入设置保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存代码注入设置失败";
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
                        自定义代码注入
                    </Heading>

                    <Tabs.Root defaultValue="head">
                        <Tabs.List>
                            <Tabs.Trigger value="head">HTML Head</Tabs.Trigger>
                            <Tabs.Trigger value="body">HTML Body</Tabs.Trigger>
                            <Tabs.Trigger value="style">Custom CSS/SCSS</Tabs.Trigger>
                        </Tabs.List>

                        <Box pt="3">
                            <Tabs.Content value="head">
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="2" style={{ display: 'block' }}>
                                        HTML Head 注入
                                    </Text>
                                    <Box style={{ border: '1px solid var(--gray-a5)', borderRadius: 'var(--radius-2)', overflow: 'hidden' }}>
                                        <Editor
                                            height="400px"
                                            defaultLanguage="html"
                                            value={form.html_head}
                                            onChange={(value) => setForm({ ...form, html_head: value || "" })}
                                            theme="light"
                                            options={{
                                                minimap: { enabled: false },
                                                scrollBeyondLastLine: false,
                                                fontSize: 14,
                                            }}
                                        />
                                    </Box>
                                    <Text size="1" color="gray" mt="1">
                                        插入到 &lt;head&gt; 标签前
                                    </Text>
                                </Box>
                            </Tabs.Content>

                            <Tabs.Content value="body">
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="2" style={{ display: 'block' }}>
                                        HTML Body 注入
                                    </Text>
                                    <Box style={{ border: '1px solid var(--gray-a5)', borderRadius: 'var(--radius-2)', overflow: 'hidden' }}>
                                        <Editor
                                            height="400px"
                                            defaultLanguage="html"
                                            value={form.html_body}
                                            onChange={(value) => setForm({ ...form, html_body: value || "" })}
                                            theme="light"
                                            options={{
                                                minimap: { enabled: false },
                                                scrollBeyondLastLine: false,
                                                fontSize: 14,
                                            }}
                                        />
                                    </Box>
                                    <Text size="1" color="gray" mt="1">
                                        插入到 &lt;/body&gt; 标签前
                                    </Text>
                                </Box>
                            </Tabs.Content>

                            <Tabs.Content value="style">
                                <Box>
                                    <Text as="label" size="2" weight="medium" mb="2" style={{ display: 'block' }}>
                                        自定义样式 (SCSS Supported)
                                    </Text>
                                    <Box style={{ border: '1px solid var(--gray-a5)', borderRadius: 'var(--radius-2)', overflow: 'hidden' }}>
                                        <Editor
                                            height="400px"
                                            defaultLanguage="scss"
                                            value={form.style}
                                            onChange={(value) => setForm({ ...form, style: value || "" })}
                                            theme="light"
                                            options={{
                                                minimap: { enabled: false },
                                                scrollBeyondLastLine: false,
                                                fontSize: 14,
                                            }}
                                        />
                                    </Box>
                                    <Text size="1" color="gray" mt="1">
                                        自定义 CSS 或 SCSS 样式，将编译后注入到页面中
                                    </Text>
                                </Box>
                            </Tabs.Content>
                        </Box>
                    </Tabs.Root>
                </Box>

                <Separator size="4" />

                <Flex justify="end" align="center" gap="3">
                    {message && (
                        <Text size="2" color={message.type === "success" ? "green" : "red"}>
                            {message.text}
                        </Text>
                    )}
                    <Button onClick={handleSave} loading={spinner.saving} disabled={spinner.saving}>保存代码注入设置</Button>
                </Flex>
            </Flex>
        </Card>
    );
};
