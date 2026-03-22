import React, { useEffect, useReducer, useState } from "react";
import { Box, Button, Card, Flex, Heading, Separator, Spinner, Text, TextArea, TextField, } from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";

export interface PostConfig {
    sensitive_words: string[];
    min_reg_age_secs: number;
    min_title_length: number;
    max_title_length: number;
    min_content_length: number;
    max_content_length: number;
    min_reply_length: number;
    max_reply_length: number;
}

const load = () => pull<PostConfig>(`/settings?name=post_config`);
const save = (value: PostConfig) => push(`/settings`, { name: "post_config", value });

export const PostSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [form, setForm] = useState<PostConfig>({
        sensitive_words: [],
        min_reg_age_secs: 0,
        min_title_length: 3,
        max_title_length: 100,
        min_content_length: 0,
        max_content_length: 1000,
        min_reply_length: 1,
        max_reply_length: 1000,
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
            setMessage({ text: "发帖设置保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存发帖设置失败";
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
                        发帖设置
                    </Heading>
                    <Flex direction="column" gap="3">
                        <Flex gap="3">
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">
                                    最小标题长度
                                </Text>
                                <TextField.Root
                                    type="number"
                                    value={String(form.min_title_length)}
                                    onChange={(e) =>
                                        setForm({ ...form, min_title_length: Number(e.target.value) })
                                    }
                                    placeholder="3"
                                />
                            </Box>
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">
                                    最大标题长度
                                </Text>
                                <TextField.Root
                                    type="number"
                                    value={String(form.max_title_length)}
                                    onChange={(e) =>
                                        setForm({ ...form, max_title_length: Number(e.target.value) })
                                    }
                                    placeholder="100"
                                />
                            </Box>
                        </Flex>

                        <Flex gap="3">
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">
                                    最小内容长度
                                </Text>
                                <TextField.Root
                                    type="number"
                                    value={String(form.min_content_length)}
                                    onChange={(e) =>
                                        setForm({ ...form, min_content_length: Number(e.target.value) })
                                    }
                                    placeholder="0"
                                />
                            </Box>
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">
                                    最大内容长度
                                </Text>
                                <TextField.Root
                                    type="number"
                                    value={String(form.max_content_length)}
                                    onChange={(e) =>
                                        setForm({ ...form, max_content_length: Number(e.target.value) })
                                    }
                                    placeholder="10000"
                                />
                            </Box>
                        </Flex>

                        <Flex gap="3">
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">
                                    最小回复长度
                                </Text>
                                <TextField.Root
                                    type="number"
                                    value={String(form.min_reply_length)}
                                    onChange={(e) =>
                                        setForm({ ...form, min_reply_length: Number(e.target.value) })
                                    }
                                    placeholder="1"
                                />
                            </Box>
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">
                                    最大回复长度
                                </Text>
                                <TextField.Root
                                    type="number"
                                    value={String(form.max_reply_length)}
                                    onChange={(e) =>
                                        setForm({ ...form, max_reply_length: Number(e.target.value) })
                                    }
                                    placeholder="1000"
                                />
                            </Box>
                        </Flex>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                最小注册时间 (秒)
                            </Text>
                            <TextField.Root
                                type="number"
                                value={String(form.min_reg_age_secs)}
                                onChange={(e) =>
                                    setForm({ ...form, min_reg_age_secs: Number(e.target.value) })
                                }
                                placeholder="0"
                            />
                            <Text size="1" color="gray" mt="1">
                                设置为 0 表示不限制
                            </Text>
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                敏感词列表
                            </Text>
                            <TextArea
                                value={form.sensitive_words.join("\n")}
                                onChange={(e) =>
                                    setForm({
                                        ...form,
                                        sensitive_words: e.target.value
                                            .split("\n")
                                            .map((s) => s.trim())
                                            .filter((s) => s.length > 0),
                                    })
                                }
                                placeholder="每行一个敏感词"
                                rows={10}
                            />
                            <Text size="1" color="gray" mt="1">
                                每行输入一个敏感词
                            </Text>
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
                    <Button onClick={handleSave} loading={spinner.saving} disabled={spinner.saving}>保存发帖设置</Button>
                </Flex>
            </Flex>
        </Card>
    );
};
