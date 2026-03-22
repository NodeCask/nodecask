import React, { useEffect, useReducer, useState } from "react";
import { Box, Button, Card, Checkbox, Flex, Heading, Separator, Spinner, Text, TextField, } from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";

export interface TurnstileConfig {
    enable: boolean;
    site_key: string;
    secret_key: string;
    hooks: string[];
}

const load = () => pull<TurnstileConfig>(`/settings?name=cloudflare_turnstile`);
const save = (value: TurnstileConfig) => push(`/settings`, { name: "cloudflare_turnstile", value });

export const TurnstileSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [form, setForm] = useState<TurnstileConfig>({
        enable: false,
        site_key: "",
        secret_key: "",
        hooks: [],
    });

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
            setMessage({ text: "Turnstile 配置保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存 Turnstile 配置失败";
            setMessage({ text: errorMessage, type: "error" });
        } finally {
            dispatch({ type: "STOP_SAVING" });
            setTimeout(() => setMessage(null), 3000);
        }
    };

    const toggleHook = (hook: string) => {
        if (form.hooks.includes(hook)) {
            setForm({ ...form, hooks: form.hooks.filter(h => h !== hook) });
        } else {
            setForm({ ...form, hooks: [...form.hooks, hook] });
        }
    };

    const availableHooks = [
        { key: "login", label: "登录" },
        { key: "password-reset", label: "重置密码" },
        { key: "register", label: "注册" },
        { key: "topic", label: "发帖" },
        { key: "comment", label: "评论" },
    ];

    return (
        <Card>
            <Flex direction="column" gap="4">
                <Box>
                    <Heading as="h3" size="4" mb="3">
                        Cloudflare Turnstile 设置
                    </Heading>
                    <Flex direction="column" gap="3">
                        <Box>
                            <Flex align="center" gap="2">
                                <Checkbox
                                    checked={form.enable}
                                    onCheckedChange={(checked) =>
                                        setForm({ ...form, enable: checked === true })
                                    }
                                />
                                <Text size="2" weight="medium">
                                    开启 Turnstile 验证
                                </Text>
                            </Flex>
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                Site Key (公钥)
                            </Text>
                            <TextField.Root
                                value={form.site_key}
                                onChange={(e) => setForm({ ...form, site_key: e.target.value })}
                                placeholder="输入 Site Key"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                Secret Key (私钥)
                            </Text>
                            <TextField.Root
                                value={form.secret_key}
                                onChange={(e) => setForm({ ...form, secret_key: e.target.value })}
                                placeholder="输入 Secret Key"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="2">
                                启用位置
                            </Text>
                            <Flex gap="4" wrap="wrap">
                                {availableHooks.map((hook) => (
                                    <Flex align="center" gap="2" key={hook.key}>
                                        <Checkbox
                                            checked={form.hooks.includes(hook.key)}
                                            onCheckedChange={() => toggleHook(hook.key)}
                                        />
                                        <Text size="2">{hook.label}</Text>
                                    </Flex>
                                ))}
                            </Flex>
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
