import React, { useReducer, useState } from "react";
import { Box, Button, Card, Flex, Heading, Separator, Text, TextField, Select } from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";
import Editor from "@monaco-editor/react";

const COMMON_CONFIGS = [
    { label: "robots.txt", value: "robots.txt" },
];

export const AdvancedSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, { ...initialSpinnerState, loading: false });
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [configName, setConfigName] = useState("");
    const [configValue, setConfigValue] = useState("");

    const handleLoad = async () => {
        if (!configName) {
            setMessage({ text: "请输入配置项名称", type: "error" });
            return;
        }
        dispatch({ type: "START_LOADING" });
        setMessage(null);
        try {
            const v = await pull<any>(`/settings?name=${configName}`);
            setConfigValue(JSON.stringify(v, null, 4));
            setMessage({ text: "加载成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "加载配置失败";
            setMessage({ text: errorMessage, type: "error" });
            setConfigValue("");
        } finally {
            dispatch({ type: "STOP_LOADING" });
            setTimeout(() => setMessage(null), 3000);
        }
    };

    const handleSave = async () => {
        if (!configName) {
            setMessage({ text: "请输入配置项名称", type: "error" });
            return;
        }
        let parsedValue;
        try {
            parsedValue = JSON.parse(configValue);
        } catch (e) {
            setMessage({ text: "JSON 格式错误", type: "error" });
            return;
        }

        dispatch({ type: "START_SAVING" });
        try {
            await push(`/settings`, { name: configName, value: parsedValue });
            setMessage({ text: "配置保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存配置失败";
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
                        高级配置
                    </Heading>

                    <Box style={{ border: '1px solid var(--gray-a5)', borderRadius: 'var(--radius-2)', overflow: 'hidden' }}>
                        <Editor
                            height="400px"
                            defaultLanguage="json"
                            value={configValue}
                            onChange={(value) => setConfigValue(value || "")}
                            theme="light"
                            options={{
                                minimap: { enabled: false },
                                scrollBeyondLastLine: false,
                                fontSize: 14,
                                formatOnPaste: true,
                                formatOnType: true,
                            }}
                        />
                    </Box>
                    <Text size="1" color="gray" mt="1">
                        请使用标准的 JSON 格式编辑配置。
                    </Text>
                </Box>

                <Separator size="4" />

                <Flex justify="between" align="center" gap="3">
                    <Flex gap="3" align="center" style={{ flexGrow: 1 }}>
                        <Select.Root value="" onValueChange={(value) => setConfigName(value)}>
                            <Select.Trigger placeholder="快速选择..." />
                            <Select.Content>
                                {COMMON_CONFIGS.map((cfg) => (
                                    <Select.Item key={cfg.value} value={cfg.value}>
                                        {cfg.label}
                                    </Select.Item>
                                ))}
                            </Select.Content>
                        </Select.Root>

                        <Box style={{ flexGrow: 1 }}>
                            <TextField.Root
                                placeholder="配置项名称 (例如: site_config)"
                                value={configName}
                                onChange={(e) => setConfigName(e.target.value)}
                            />
                        </Box>

                        <Button onClick={handleLoad} loading={spinner.loading} disabled={spinner.loading}>
                            加载
                        </Button>
                    </Flex>

                    <Flex align="center" gap="3">
                        {message && (
                            <Text size="2" color={message.type === "success" ? "green" : "red"}>
                                {message.text}
                            </Text>
                        )}
                        <Button onClick={handleSave} variant="solid" color="blue" loading={spinner.saving} disabled={spinner.saving || spinner.loading}>
                            保存配置
                        </Button>
                    </Flex>
                </Flex>
            </Flex>
        </Card>
    );
};