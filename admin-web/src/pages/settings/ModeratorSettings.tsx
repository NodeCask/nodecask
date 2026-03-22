import React, { useEffect, useReducer, useState } from "react";
import {
    Badge,
    Box,
    Button,
    Callout,
    Dialog,
    Flex,
    Heading,
    IconButton,
    Separator,
    Spinner,
    Switch,
    Table,
    Tabs,
    Text,
    TextArea,
    TextField,
    Strong,
    Code,
    Select,
} from "@radix-ui/themes";
import { pull, push, SysError } from "../../api";
import { InfoCircledIcon, Pencil1Icon, PlusIcon, TrashIcon } from "@radix-ui/react-icons";
import { initialSpinnerState, spinnerReducer } from "../../hooks";
import prompt from "./prompt.txt?raw";

export interface PromptConfig {
    system: string;
    user: string;
}

export interface Model {
    name: string; // name 不能重复
    url: string;
    key: string;
    model: string;
    temperature: number;
    max_content: number;
    flavor: "google" | "openai";
}

export interface Moderator {
    name: string;
    model: string; // 对应上面 Model 类型 name 字段
    enable: boolean;
    target: "topic" | "comment";
    prompt: PromptConfig;
    credit_damage: number;
    coins_damage: number;
}

export interface ModeratorConfig {
    models: Model[];
    moderators: Moderator[];
}

const load = () => pull<ModeratorConfig>(`/settings?name=llm_moderator`);
const save = (value: ModeratorConfig) => push(`/settings`, { name: "llm_moderator", value });

const DEFAULT_PROMPT = {
    system: prompt,
    user: "",
};
// Default prompt templates for easier start
const DEFAULT_TOPIC_PROMPT = {
    system: prompt,
    user: "<user_input>标题：%TOPIC_TITLE%\n内容：%TOPIC_CONTENT_PLAIN%</user_input>",
};
const DEFAULT_COMMENT_PROMPT = {
    system: prompt,
    user: "<user_input>%COMMENT_CONTENT_PLAIN%</user_input>",
};


const DEFAULT_OPENAI_URL = "https://api.openai.com/v1/chat/completions";
const DEFAULT_GOOGLE_URL = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite:generateContent";

const DEFAULT_MODEL: Model = {
    name: "New Model",
    url: DEFAULT_OPENAI_URL,
    key: "",
    model: "gpt-3.5-turbo",
    temperature: 0.7,
    max_content: 2000,
    flavor: "openai",
};

const DEFAULT_MODERATOR: Moderator = {
    name: "New Rule",
    model: "",
    enable: false,
    target: "topic",
    prompt: DEFAULT_PROMPT,
    credit_damage: 0,
    coins_damage: 0,
};

const DEFAULT_CONFIG: ModeratorConfig = {
    models: [],
    moderators: [],
};

export const ModeratorSettings: React.FC = () => {
    const [spinner, dispatch] = useReducer(spinnerReducer, initialSpinnerState);
    const [config, setConfig] = useState<ModeratorConfig>(DEFAULT_CONFIG);
    const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);

    // Editing State
    const [modelDialogOpen, setModelDialogOpen] = useState(false);
    const [modDialogOpen, setModDialogOpen] = useState(false);
    const [editingModelIndex, setEditingModelIndex] = useState(-1);
    const [editingModIndex, setEditingModIndex] = useState(-1);

    const [editModel, setEditModel] = useState<Model>(DEFAULT_MODEL);
    const [editMod, setEditMod] = useState<Moderator>(DEFAULT_MODERATOR);

    useEffect(() => {
        load().then((data) => {
            // Handle case where data might be legacy array or null
            if (data && typeof data === 'object' && 'models' in data) {
                setConfig(data);
            } else {
                setConfig(DEFAULT_CONFIG);
            }
        }).catch(() => {
            setConfig(DEFAULT_CONFIG);
        }).finally(() => dispatch({ type: "STOP_LOADING" }));
    }, []);

    const handleSaveSettings = async () => {
        dispatch({ type: "START_SAVING" });
        try {
            await save(config);
            setMessage({ text: "审核设置保存成功", type: "success" });
        } catch (err) {
            const errorMessage =
                err instanceof SysError ? err.message : "保存审核设置失败";
            setMessage({ text: errorMessage, type: "error" });
        } finally {
            dispatch({ type: "STOP_SAVING" });
            setTimeout(() => setMessage(null), 3000);
        }
    };

    // --- Model Handlers ---

    const handleAddModel = () => {
        setEditingModelIndex(-1);
        setEditModel({ ...DEFAULT_MODEL });
        setModelDialogOpen(true);
    };

    const handleEditModel = (index: number) => {
        setEditingModelIndex(index);
        setEditModel({ ...config.models[index] });
        setModelDialogOpen(true);
    };

    const handleDeleteModel = (index: number) => {
        if (confirm("确定要删除这个模型吗？引用的审核规则可能会失效。")) {
            const newModels = [...config.models];
            newModels.splice(index, 1);
            setConfig({ ...config, models: newModels });
        }
    };

    const handleSaveModel = () => {
        // Name duplicate check
        const nameExists = config.models.some((m, idx) => m.name === editModel.name && idx !== editingModelIndex);
        if (nameExists) {
            alert("模型名称已存在，请使用其他名称");
            return;
        }

        const newModels = [...config.models];
        if (editingModelIndex === -1) {
            newModels.push(editModel);
        } else {
            newModels[editingModelIndex] = editModel;
        }
        setConfig({ ...config, models: newModels });
        setModelDialogOpen(false);
    };

    // --- Moderator Handlers ---

    const handleAddMod = () => {
        setEditingModIndex(-1);
        setEditMod({
            ...DEFAULT_MODERATOR,
            prompt: DEFAULT_TOPIC_PROMPT, // Default to topic prompt
            model: config.models.length > 0 ? config.models[0].name : ""
        });
        setModDialogOpen(true);
    };

    const handleEditMod = (index: number) => {
        setEditingModIndex(index);
        setEditMod({ ...config.moderators[index] });
        setModDialogOpen(true);
    };

    const handleDeleteMod = (index: number) => {
        if (confirm("确定要删除这个审核规则吗？")) {
            const newMods = [...config.moderators];
            newMods.splice(index, 1);
            setConfig({ ...config, moderators: newMods });
        }
    };

    const handleSaveMod = () => {
        const newMods = [...config.moderators];
        if (editingModIndex === -1) {
            newMods.push(editMod);
        } else {
            newMods[editingModIndex] = editMod;
        }
        setConfig({ ...config, moderators: newMods });
        setModDialogOpen(false);
    };

    // Changing target updates default prompt if user hasn't typed much (simple heuristic or just force it?)
    // Let's just update prompt if user confirms or switches? For now just simple switch target.
    const handleTargetChange = (val: "topic" | "comment") => {
        setEditMod(prev => ({
            ...prev,
            target: val,
            prompt: val === 'topic' ? DEFAULT_TOPIC_PROMPT : DEFAULT_COMMENT_PROMPT
        }));
    };


    if (spinner.loading) {
        return (
            <Flex align="center" justify="center" py="9">
                <Spinner size="3" />
            </Flex>
        );
    }

    const handleFlavorChange = (val: "openai" | "google") => {
        let newUrl = editModel.url;
        // Check if current URL is one of the defaults (or empty/legacy default)
        // If so, switch to the new flavor's default.
        // If user has a custom URL, we probably shouldn't touch it, 
        // OR we could argue if it's "not the new default", we ask? 
        // The requirement says: "If it's default unmodified, switch it."
        const isDefaultOpenAI = editModel.url === DEFAULT_OPENAI_URL;
        const isDefaultGoogle = editModel.url === DEFAULT_GOOGLE_URL;

        if (val === "openai") {
            if (isDefaultGoogle || editModel.url === "") {
                newUrl = DEFAULT_OPENAI_URL;
            }
        } else if (val === "google") {
            if (isDefaultOpenAI || editModel.url === "") {
                newUrl = DEFAULT_GOOGLE_URL;
            }
        }
        setEditModel({ ...editModel, flavor: val, url: newUrl });
    };

    return (
        <Flex direction="column" gap="4">
            <Box>
                <Heading as="h3" size="4" mb="3">
                    LLM 内容审核
                </Heading>
                <Text size="2" color="gray" mb="4" as="p">
                    定义 LLM 模型，并配置针对帖子或评论的审核规则。
                </Text>

                <Tabs.Root defaultValue="moderators">
                    <Tabs.List>
                        <Tabs.Trigger value="moderators">审核规则 (Moderators)</Tabs.Trigger>
                        <Tabs.Trigger value="models">模型配置 (Models)</Tabs.Trigger>
                    </Tabs.List>

                    <Box pt="3">
                        <Tabs.Content value="models">
                            <Table.Root variant="surface">
                                <Table.Header>
                                    <Table.Row>
                                        <Table.ColumnHeaderCell>名称</Table.ColumnHeaderCell>
                                        <Table.ColumnHeaderCell>Flavor</Table.ColumnHeaderCell>
                                        <Table.ColumnHeaderCell>模型</Table.ColumnHeaderCell>
                                        <Table.ColumnHeaderCell>API URL</Table.ColumnHeaderCell>
                                        <Table.ColumnHeaderCell>操作</Table.ColumnHeaderCell>
                                    </Table.Row>
                                </Table.Header>
                                <Table.Body>
                                    {config.models.map((m, i) => (
                                        <Table.Row key={i}>
                                            <Table.Cell><Strong>{m.name}</Strong></Table.Cell>
                                            <Table.Cell>{m.flavor}</Table.Cell>
                                            <Table.Cell>{m.model}</Table.Cell>
                                            <Table.Cell>
                                                <Text size="1" color="gray" style={{ maxWidth: 200, overflow: 'hidden', textOverflow: 'ellipsis', display: 'block' }}>{m.url}</Text>
                                            </Table.Cell>
                                            <Table.Cell>
                                                <Flex gap="3">
                                                    <IconButton variant="ghost" onClick={() => handleEditModel(i)}><Pencil1Icon /></IconButton>
                                                    <IconButton variant="ghost" color="red" onClick={() => handleDeleteModel(i)}><TrashIcon /></IconButton>
                                                </Flex>
                                            </Table.Cell>
                                        </Table.Row>
                                    ))}
                                    {config.models.length === 0 && <Table.Row><Table.Cell colSpan={5} align="center">暂无模型配置</Table.Cell></Table.Row>}
                                </Table.Body>
                            </Table.Root>
                            <Box mt="3">
                                <Button variant="surface" onClick={handleAddModel}><PlusIcon /> 添加模型</Button>
                            </Box>
                        </Tabs.Content>

                        <Tabs.Content value="moderators">
                            <Table.Root variant="surface">
                                <Table.Header>
                                    <Table.Row>
                                        <Table.ColumnHeaderCell>规则名称</Table.ColumnHeaderCell>
                                        <Table.ColumnHeaderCell>状态</Table.ColumnHeaderCell>
                                        <Table.ColumnHeaderCell>目标</Table.ColumnHeaderCell>
                                        <Table.ColumnHeaderCell>使用模型</Table.ColumnHeaderCell>
                                        <Table.ColumnHeaderCell>操作</Table.ColumnHeaderCell>
                                    </Table.Row>
                                </Table.Header>
                                <Table.Body>
                                    {config.moderators.map((m, i) => (
                                        <Table.Row key={i}>
                                            <Table.Cell><Strong>{m.name}</Strong></Table.Cell>
                                            <Table.Cell>
                                                <Badge color={m.enable ? "green" : "gray"}>{m.enable ? "启用" : "禁用"}</Badge>
                                            </Table.Cell>
                                            <Table.Cell>
                                                <Badge variant="outline">{m.target === 'topic' ? '帖子' : '评论'}</Badge>
                                            </Table.Cell>
                                            <Table.Cell>
                                                {config.models.some(chk => chk.name === m.model) ?
                                                    <Text>{m.model}</Text> :
                                                    <Text color="red">无效模型 ({m.model})</Text>
                                                }
                                            </Table.Cell>
                                            <Table.Cell>
                                                <Flex gap="3">
                                                    <IconButton variant="ghost" onClick={() => handleEditMod(i)}><Pencil1Icon /></IconButton>
                                                    <IconButton variant="ghost" color="red" onClick={() => handleDeleteMod(i)}><TrashIcon /></IconButton>
                                                </Flex>
                                            </Table.Cell>
                                        </Table.Row>
                                    ))}
                                    {config.moderators.length === 0 && <Table.Row><Table.Cell colSpan={5} align="center">暂无审核规则</Table.Cell></Table.Row>}
                                </Table.Body>
                            </Table.Root>
                            <Box mt="3">
                                <Button variant="surface" onClick={handleAddMod}><PlusIcon /> 添加审核规则</Button>
                            </Box>

                            <Callout.Root color="blue" mt="4">
                                <Callout.Icon><InfoCircledIcon /></Callout.Icon>
                                <Callout.Text>
                                    提示：请先在“模型配置”中添加 LLM 模型，然后在“审核规则”中引用该模型。
                                    <br />
                                    变量支持: <Code>%TOPIC_TITLE%</Code> <Code>%TOPIC_CONTENT_PLAIN%</Code> <Code>%COMMENT_CONTENT_PLAIN%</Code> 等。
                                </Callout.Text>
                            </Callout.Root>
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
                <Button onClick={handleSaveSettings} loading={spinner.saving} disabled={spinner.saving}>保存审核设置</Button>
            </Flex>


            {/* Model Dialog */}
            <Dialog.Root open={modelDialogOpen} onOpenChange={setModelDialogOpen}>
                <Dialog.Content style={{ maxWidth: 600 }}>
                    <Dialog.Title>{editingModelIndex === -1 ? "添加模型" : "编辑模型"}</Dialog.Title>
                    <Flex direction="column" gap="3">
                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">模型配置名称 (唯一)</Text>
                            <TextField.Root value={editModel.name} onChange={e => setEditModel({ ...editModel, name: e.target.value })} placeholder="例如：GPT-4o" />
                        </Box>
                        <Flex gap="3">
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">Flavor</Text>
                                <Select.Root value={editModel.flavor} onValueChange={(v: any) => handleFlavorChange(v)}>
                                    <Select.Trigger style={{ width: '100%' }} />
                                    <Select.Content>
                                        <Select.Item value="openai">OpenAI Compatible</Select.Item>
                                        <Select.Item value="google">Google Gemini</Select.Item>
                                    </Select.Content>
                                </Select.Root>
                            </Box>
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">Model ID</Text>
                                <TextField.Root value={editModel.model} onChange={e => setEditModel({ ...editModel, model: e.target.value })} placeholder="例如：gpt-4o" />
                            </Box>
                        </Flex>
                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">API URL</Text>
                            <TextField.Root value={editModel.url} onChange={e => setEditModel({ ...editModel, url: e.target.value })} />
                        </Box>
                        <Box>
                            <Text as="label" size="2" weight="medium" mb="1">API Key</Text>
                            <TextField.Root type="text" value={editModel.key} onChange={e => setEditModel({ ...editModel, key: e.target.value })} />
                        </Box>
                        <Flex gap="3">
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">Temperature</Text>
                                <TextField.Root type="number" step="0.1" value={String(editModel.temperature)} onChange={e => setEditModel({ ...editModel, temperature: Number(e.target.value) })} />
                            </Box>
                            <Box flexGrow="1">
                                <Text as="label" size="2" weight="medium" mb="1">Max Tokens / Length</Text>
                                <TextField.Root type="number" value={String(editModel.max_content)} onChange={e => setEditModel({ ...editModel, max_content: Number(e.target.value) })} />
                            </Box>
                        </Flex>
                    </Flex>
                    <Flex gap="3" mt="4" justify="end">
                        <Dialog.Close><Button variant="soft" color="gray">取消</Button></Dialog.Close>
                        <Button onClick={handleSaveModel}>确定</Button>
                    </Flex>
                </Dialog.Content>
            </Dialog.Root>

            {/* Moderator Dialog */}
            <Dialog.Root open={modDialogOpen} onOpenChange={setModDialogOpen}>
                <Dialog.Content style={{ maxWidth: 800 }}>
                    <Dialog.Title>{editingModIndex === -1 ? "添加审核规则" : "编辑审核规则"}</Dialog.Title>
                    <Tabs.Root defaultValue="basic">
                        <Tabs.List>
                            <Tabs.Trigger value="basic">基本设置</Tabs.Trigger>
                            <Tabs.Trigger value="prompt">Prompt 设置</Tabs.Trigger>
                            <Tabs.Trigger value="punish">惩罚设置</Tabs.Trigger>
                        </Tabs.List>
                        <Box pt="3">
                            <Tabs.Content value="basic">
                                <Flex direction="column" gap="3">
                                    <Flex gap="3">
                                        <Box flexGrow="1">
                                            <Text as="label" size="2" weight="medium" mb="1">规则名称</Text>
                                            <TextField.Root value={editMod.name} onChange={e => setEditMod({ ...editMod, name: e.target.value })} placeholder="例如：涉政审核" />
                                        </Box>
                                        <Box>
                                            <Text as="label" size="2" weight="medium" mb="1" style={{ display: 'block' }}>状态</Text>
                                            <Flex align="center" gap="2" height="32px">
                                                <Switch checked={editMod.enable} onCheckedChange={c => setEditMod({ ...editMod, enable: c })} />
                                                <Text size="2">{editMod.enable ? "启用" : "禁用"}</Text>
                                            </Flex>
                                        </Box>
                                    </Flex>
                                    <Flex gap="3">
                                        <Box flexGrow="1">
                                            <Text as="label" size="2" weight="medium" mb="1">审核目标</Text>
                                            <Select.Root value={editMod.target} onValueChange={(v: any) => handleTargetChange(v)}>
                                                <Select.Trigger style={{ width: '100%' }} />
                                                <Select.Content>
                                                    <Select.Item value="topic">新发帖子</Select.Item>
                                                    <Select.Item value="comment">新发评论</Select.Item>
                                                </Select.Content>
                                            </Select.Root>
                                        </Box>
                                        <Box flexGrow="1">
                                            <Text as="label" size="2" weight="medium" mb="1">使用模型</Text>
                                            <Select.Root value={editMod.model} onValueChange={v => setEditMod({ ...editMod, model: v })}>
                                                <Select.Trigger style={{ width: '100%' }} placeholder="选择模型..." />
                                                <Select.Content>
                                                    {config.models.map(m => (
                                                        <Select.Item key={m.name} value={m.name}>{m.name} ({m.model})</Select.Item>
                                                    ))}
                                                </Select.Content>
                                            </Select.Root>
                                        </Box>
                                    </Flex>
                                </Flex>
                            </Tabs.Content>
                            <Tabs.Content value="prompt">
                                <Flex direction="column" gap="3">
                                    <Box>
                                        <Text as="label" size="2" weight="medium" mb="1">System Prompt</Text>
                                        <TextArea value={editMod.prompt.system} onChange={e => setEditMod({ ...editMod, prompt: { ...editMod.prompt, system: e.target.value } })} rows={5} />
                                    </Box>
                                    <Box>
                                        <Text as="label" size="2" weight="medium" mb="1">User Prompt</Text>
                                        <TextArea value={editMod.prompt.user} onChange={e => setEditMod({ ...editMod, prompt: { ...editMod.prompt, user: e.target.value } })} rows={8} style={{ fontFamily: 'monospace', fontSize: 13 }} />
                                    </Box>
                                </Flex>
                            </Tabs.Content>
                            <Tabs.Content value="punish">
                                <Flex direction="column" gap="3">
                                    <Callout.Root color="gray" size="1">
                                        <Callout.Text>当审核结果为 <Code>delete</Code> (删除) 时，执行以下惩罚。</Callout.Text>
                                    </Callout.Root>
                                    <Flex gap="3">
                                        <Box flexGrow="1">
                                            <Text as="label" size="2" weight="medium" mb="1">删帖扣除信誉</Text>
                                            <TextField.Root type="number" value={String(editMod.credit_damage)} onChange={e => setEditMod({ ...editMod, credit_damage: Number(e.target.value) })} />
                                        </Box>
                                        <Box flexGrow="1">
                                            <Text as="label" size="2" weight="medium" mb="1">删帖扣除金币</Text>
                                            <TextField.Root type="number" value={String(editMod.coins_damage)} onChange={e => setEditMod({ ...editMod, coins_damage: Number(e.target.value) })} />
                                        </Box>
                                    </Flex>
                                </Flex>
                            </Tabs.Content>
                        </Box>
                    </Tabs.Root>

                    <Flex gap="3" mt="4" justify="end">
                        <Dialog.Close><Button variant="soft" color="gray">取消</Button></Dialog.Close>
                        <Button onClick={handleSaveMod}>确定</Button>
                    </Flex>
                </Dialog.Content>
            </Dialog.Root>

        </Flex>
    );
};