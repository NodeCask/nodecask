import React, { useEffect, useReducer, useState } from "react";
import { Box, Button, Callout, Card, Code, Flex, Heading, Separator, Spinner, Text, } from "@radix-ui/themes";
import { InfoCircledIcon } from "@radix-ui/react-icons";
import { pull, push, SysError } from "../../api";
import { initialSpinnerState, spinnerReducer } from "../../hooks";
import Editor from "@monaco-editor/react";

export interface LoginRewardsConfig {
    script: string;
}

const load = () => pull<LoginRewardsConfig>(`/settings?name=login_rewards_script`);
const save = (value: LoginRewardsConfig) => push(`/settings`, { name: "login_rewards_script", value });

export const LoginRewardsSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
    const [form, setForm] = useState<LoginRewardsConfig>({
        script: "",
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
            setMessage({ text: "签到奖励规则保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存签到奖励规则失败";
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
                        签到奖励规则
                    </Heading>

                    <Callout.Root color="blue">
                        <Callout.Icon>
                            <InfoCircledIcon />
                        </Callout.Icon>
                        <Callout.Text>
                            你需要定义两个函数：<Code>calculateLoginRewards(data)</Code> 和 <Code>calculateCheckInRewards(data)</Code>，返回 <Code>LoginRewards</Code> 对象。
                        </Callout.Text>
                    </Callout.Root>

                    <Flex direction="column" gap="3" mt="3">
                        <Box>
                            <Text as="label" size="2" weight="medium" mb="2" style={{ display: 'block' }}>
                                奖励脚本 (JavaScript)
                            </Text>
                            <Box style={{
                                border: '1px solid var(--gray-a5)',
                                borderRadius: 'var(--radius-2)',
                                overflow: 'hidden'
                            }}>
                                <Editor
                                    height="400px"
                                    defaultLanguage="javascript"
                                    value={form.script}
                                    onChange={(value) => setForm({ ...form, script: value || "" })}
                                    theme="light"
                                    options={{
                                        minimap: { enabled: false },
                                        scrollBeyondLastLine: false,
                                        fontSize: 14,
                                    }}
                                />
                            </Box>
                        </Box>
                    </Flex>

                    <Box mt="3">
                        <Text size="2" weight="bold" mb="2" as="div">数据结构参考</Text>
                        <Code style={{ display: "block", padding: "10px", whiteSpace: "pre-wrap" }} variant="soft">
                            {`// 会员连续登录数据
interface AccessDaysData {
    LastAccessDate: string; // 最后访问日期
    AccessDays: number; // 累计登录天数
    ContinuousAccessDays: number;// 连续登录天数
}

// 会员连续打卡数据
interface CheckInData {
    last_checkin_date: string // 最后打卡日期
    total_checkin_count: number // 累计打卡天数
    current_continuous_checkin_count: number // 连续打卡天数
}

// 登录或是打卡奖励
interface LoginRewards {
    credit: number; // 信誉分
    coins: number;  // 金币
}

// 函数签名
function calculateLoginRewards(data: AccessDaysData): LoginRewards { ... }
function calculateCheckInRewards(data: CheckInData): LoginRewards { ... }
`}
                        </Code>
                    </Box>
                </Box>

                <Separator size="4" />

                <Flex justify="end" align="center" gap="3">
                    {message && (
                        <Text size="2" color={message.type === "success" ? "green" : "red"}>
                            {message.text}
                        </Text>
                    )}
                    <Button onClick={handleSave} loading={spinner.saving} disabled={spinner.saving}>保存签到奖励规则</Button>
                </Flex>
            </Flex>
        </Card>
    );
};
