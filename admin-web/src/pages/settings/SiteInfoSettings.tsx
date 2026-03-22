import React, { useEffect, useReducer, useState } from "react";
import { Box, Button, Card, Flex, Heading, Separator, Spinner, Text, TextArea, TextField, } from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";

export interface SiteInfoConfig {
    name: string;
    nickname: string;
    domain: string;
    description: string;
    keyword: string;
    copyright: string;
    about: string;
}

const load = () => pull<SiteInfoConfig>(`/settings?name=site_info`);
const save = (value: SiteInfoConfig) => push(`/settings`, { name: "site_info", value });


export const SiteInfoSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [form, setForm] = useState<SiteInfoConfig>({
        name: "",
        nickname: "",
        domain: "",
        description: "",
        keyword: "",
        copyright: "",
        about: "",
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
            setMessage({ text: "网站信息保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存网站信息失败";
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
                        网站基本信息
                    </Heading>
                    <Flex direction="column" gap="3">
                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                网站名称
                            </Text>
                            <TextField.Root
                                value={form.name}
                                onChange={(e) => setForm({ ...form, name: e.target.value })}
                                placeholder="输入网站名称"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                网站名称缩写
                            </Text>
                            <TextField.Root
                                value={form.nickname}
                                onChange={(e) => setForm({ ...form, nickname: e.target.value })}
                                placeholder="输入网站名称缩写"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                网站域名
                            </Text>
                            <TextField.Root
                                value={form.domain}
                                onChange={(e) => setForm({ ...form, domain: e.target.value })}
                                placeholder="https://example.com"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                网站描述
                            </Text>
                            <TextArea
                                value={form.description}
                                onChange={(e) =>
                                    setForm({ ...form, description: e.target.value })
                                }
                                placeholder="输入网站描述"
                                rows={3}
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                网站关键词
                            </Text>
                            <TextField.Root
                                value={form.keyword}
                                onChange={(e) => setForm({ ...form, keyword: e.target.value })}
                                placeholder="输入关键词，用逗号分隔"
                            />
                            <Text size="1" color="gray" mt="1">
                                多个关键词请用英文逗号分隔
                            </Text>
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                版权声明
                            </Text>
                            <TextField.Root
                                value={form.copyright}
                                onChange={(e) => setForm({ ...form, copyright: e.target.value })}
                                placeholder="Copyright © 2026 All rights Reserved"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                关于我们
                            </Text>
                            <TextArea
                                value={form.about}
                                onChange={(e) => setForm({ ...form, about: e.target.value })}
                                placeholder="输入关于我们内容"
                                rows={5}
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
                    <Button onClick={handleSave} loading={spinner.saving} disabled={spinner.saving}>保存网站信息</Button>
                </Flex>
            </Flex>
        </Card>
    );
};
