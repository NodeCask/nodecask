import React, { useEffect, useReducer, useState } from "react";
import { Box, Button, Card, Checkbox, Flex, Heading, Separator, Spinner, Text, TextArea, TextField, } from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";

export interface RegisterConfig {
    enable: boolean;
    email_verify: boolean;
    invite_code_required: boolean;
    min_username: number;
    max_username: number;
    min_password_len: number;
    terms: string;
    initial_score: number;
    initial_avatar: string;
    reserved_username: string[];
    reserved_prefix: string[];
    reserved_suffix: string[];
}

const load = () => pull<RegisterConfig>(`/settings?name=register_config`);
const save = (value: RegisterConfig) => push(`/settings`, { name: "register_config", value });

export const RegisterSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [form, setForm] = useState<RegisterConfig>({
        enable: false,
        email_verify: false,
        invite_code_required: false,
        min_username: 3,
        max_username: 20,
        min_password_len: 6,
        terms: "",
        initial_score: 100,
        initial_avatar: "",
        reserved_username: [],
        reserved_prefix: [],
        reserved_suffix: [],
    });

    useEffect(() => {
        load()
            .then(v => {
                if(v){
                    if(!Array.isArray(v.reserved_username)) v.reserved_username = [];
                    if(!Array.isArray(v.reserved_prefix)) v.reserved_prefix = [];
                    if(!Array.isArray(v.reserved_suffix)) v.reserved_suffix = [];
                    setForm(v)
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
            setMessage({ text: "注册设置保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存注册设置失败";
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
                        用户注册设置
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
                                    开启用户注册
                                </Text>
                            </Flex>
                        </Box>
                        <Box>
                            <Flex align="center" gap="2">
                                <Checkbox
                                    checked={form.email_verify}
                                    onCheckedChange={(checked) =>
                                        setForm({ ...form, email_verify: checked === true })
                                    }
                                />
                                <Text size="2" weight="medium">
                                    开启邮箱验证
                                </Text>
                            </Flex>
                        </Box>
                        <Box>
                            <Flex align="center" gap="2">
                                <Checkbox
                                    checked={form.invite_code_required}
                                    onCheckedChange={(checked) =>
                                        setForm({ ...form, invite_code_required: checked === true })
                                    }
                                />
                                <Text size="2" weight="medium">
                                    注册需要邀请码
                                </Text>
                            </Flex>
                        </Box>
                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                最短密码长度
                            </Text>
                            <TextField.Root
                                type="number"
                                value={String(form.min_password_len)}
                                onChange={(e) =>
                                    setForm({ ...form, min_password_len: Number(e.target.value) })
                                }
                                placeholder="6"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                最小用户名长度
                            </Text>
                            <TextField.Root
                                type="number"
                                value={String(form.min_username)}
                                onChange={(e) =>
                                    setForm({ ...form, min_username: Number(e.target.value) })
                                }
                                placeholder="3"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                最大用户名长度
                            </Text>
                            <TextField.Root
                                type="number"
                                value={String(form.max_username)}
                                onChange={(e) =>
                                    setForm({ ...form, max_username: Number(e.target.value) })
                                }
                                placeholder="20"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                初始信誉分数
                            </Text>
                            <TextField.Root
                                type="number"
                                value={String(form.initial_score)}
                                onChange={(e) =>
                                    setForm({ ...form, initial_score: Number(e.target.value) })
                                }
                                placeholder="100"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                默认头像
                            </Text>
                            <Box mb="2">
                                <input
                                    type="file"
                                    accept="image/*"
                                    onChange={(e) => {
                                        const file = e.target.files?.[0];
                                        if (file) {
                                            const reader = new FileReader();
                                            reader.onloadend = () => {
                                                setForm({
                                                    ...form,
                                                    initial_avatar: reader.result as string,
                                                });
                                            };
                                            reader.readAsDataURL(file);
                                        }
                                    }}
                                />
                            </Box>
                            {form.initial_avatar && (
                                <Flex direction="column" gap="2" align="start">
                                    <img
                                        src={form.initial_avatar}
                                        alt="Avatar Preview"
                                        style={{
                                            maxWidth: "100px",
                                            maxHeight: "100px",
                                            borderRadius: "4px",
                                        }}
                                        onError={(e) => (e.currentTarget.style.display = "none")}
                                    />
                                    <Button
                                        variant="soft"
                                        color="red"
                                        size="1"
                                        onClick={() => setForm({ ...form, initial_avatar: "" })}
                                    >
                                        清除图片
                                    </Button>
                                </Flex>
                            )}
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                保留用户名
                            </Text>
                            <TextArea
                                value={form.reserved_username.join("\n")}
                                onChange={(e) =>
                                    setForm({
                                        ...form,
                                        reserved_username: e.target.value
                                            .split("\n")
                                            .map((s) => s.trim())
                                            .filter((s) => s.length > 0),
                                    })
                                }
                                placeholder="输入不允许注册的用户名"
                                rows={5}
                            />
                            <Text size="1" color="gray" mt="1">
                                每行输入一个用户名，忽略大小写
                            </Text>
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                保留前缀
                            </Text>
                            <TextArea
                                value={form.reserved_prefix.join("\n")}
                                onChange={(e) =>
                                    setForm({
                                        ...form,
                                        reserved_prefix: e.target.value
                                            .split("\n")
                                            .map((s) => s.trim())
                                            .filter((s) => s.length > 0),
                                    })
                                }
                                placeholder="输入不允许注册的域名前缀 (例如 admin)"
                                rows={5}
                            />
                            <Text size="1" color="gray" mt="1">
                                如果填写 "admin"，则 "admin123" 将无法注册
                            </Text>
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                保留后缀
                            </Text>
                            <TextArea
                                value={form.reserved_suffix.join("\n")}
                                onChange={(e) =>
                                    setForm({
                                        ...form,
                                        reserved_suffix: e.target.value
                                            .split("\n")
                                            .map((s) => s.trim())
                                            .filter((s) => s.length > 0),
                                    })
                                }
                                placeholder="输入不允许注册的域名后缀 (例如 _bot)"
                                rows={5}
                            />
                            <Text size="1" color="gray" mt="1">
                                如果填写 "_bot"，则 "ai_bot" 将无法注册
                            </Text>
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                注册条款 (HTML)
                            </Text>
                            <TextArea
                                value={form.terms}
                                onChange={(e) => setForm({ ...form, terms: e.target.value })}
                                placeholder="输入注册条款内容，支持 HTML"
                                rows={10}
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
                    <Button onClick={handleSave} loading={spinner.saving} disabled={spinner.saving}>保存注册设置</Button>
                </Flex>
            </Flex>
        </Card>
    );
};
