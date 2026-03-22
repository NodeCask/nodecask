import React, { useEffect, useReducer, useState } from "react";
import { Box, Button, Card, Flex, Heading, Separator, Spinner, Text, TextField, } from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";

export interface VisitConfig {
    topics_per_page: number;
    comments_per_page: number;
}

const load = () => pull<VisitConfig>(`/settings?name=visit_config`);
const save = (value: VisitConfig) => push(`/settings`, { name: "visit_config", value });


export const VisitSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [form, setForm] = useState<VisitConfig>({
        topics_per_page: 20,
        comments_per_page: 100,
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
            setMessage({ text: "浏览设定保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存浏览设定失败";
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
                        浏览设定
                    </Heading>
                    <Flex direction="column" gap="3">
                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                每页展示帖子数量
                            </Text>
                            <TextField.Root
                                type="number"
                                value={form.topics_per_page}
                                onChange={(e) => setForm({ ...form, topics_per_page: parseInt(e.target.value) || 0 })}
                                placeholder="20"
                            />
                        </Box>

                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">
                                帖子内每页展示回复数量
                            </Text>
                            <TextField.Root
                                type="number"
                                value={form.comments_per_page}
                                onChange={(e) => setForm({ ...form, comments_per_page: parseInt(e.target.value) || 0 })}
                                placeholder="100"
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
                    <Button onClick={handleSave} loading={spinner.saving} disabled={spinner.saving}>保存浏览设定</Button>
                </Flex>
            </Flex>
        </Card>
    );
};
