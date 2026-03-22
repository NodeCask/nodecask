import React, {useEffect, useState} from "react";
import {Box, Card, Container, Flex, Grid, Heading, Separator, Text,} from "@radix-ui/themes";
import {useNavigate} from "react-router";
import {formatDuration, formatFileSize, formatPercentage} from "../utils/formatters";
import {pull} from "../api";

export interface DashboardData {
    hostname: string;
    os_info: string;
    content_stats: {
        user_count: number;
        topic_count: number;
        comment_count: number;
    };
    disks: {
        name: string;
        mount_point: string;
        total_space: number;
        available_space: number;
        file_system: string;
    }[];
    sqlite_version: string;
    build_os: string;
    build_time: string;
    rust_version: string;
    available_memory: number;
    total_memory: number;
    used_memory: number;
    boot_time: number;
}

async function getDashboard(): Promise<DashboardData> {
    return pull<DashboardData>("/dashboard");
}

const HomePage: React.FC = () => {
    const [data, setData] = useState<DashboardData | null>(null);
    const navigate = useNavigate();

    useEffect(() => {
        getDashboard().then(setData).catch(console.error);
    }, []);

    // 统计数据
    const stats = data
        ? [
            {
                label: "用户总数",
                value: data.content_stats.user_count.toLocaleString(),
                color: "blue",
            },
            {
                label: "帖子总数",
                value: data.content_stats.topic_count.toLocaleString(),
                color: "green",
            },
            {
                label: "评论总数",
                value: data.content_stats.comment_count.toLocaleString(),
                color: "orange",
            },
        ]
        : [];

    // 系统信息
    const systemInfo = data
        ? [
            {label: "主机名", value: data.hostname},
            {label: "系统信息", value: data.os_info},
            {label: "编译系统", value: data.build_os},
            {label: "编译时间", value: data.build_time},
            {label: "Rust 版本", value: data.rust_version},
            {
                label: "系统内存",
                value: `${formatFileSize(data.used_memory)} / ${formatFileSize(data.total_memory)} (${formatPercentage(data.used_memory, data.total_memory)})`
            },
            {
                label: "运行时间",
                value: formatDuration(Math.floor(Date.now() / 1000) - data.boot_time)
            },
            ...data.disks.map((disk) => ({
                label: `磁盘 ${disk.mount_point}`,
                value: `${formatFileSize(disk.available_space)} / ${formatFileSize(
                    disk.total_space
                )} (${disk.file_system})`,
            })),
            {label: "SQLite 驱动版本", value: data.sqlite_version}
        ]
        : [];

    // 快速操作
    const quickActions = [
        {label: "用户管理", description: "管理用户账号和权限", path: "/users"},
        {label: "帖子管理", description: "审核和管理论坛帖子", path: "/posts"},
        {
            label: "分类管理",
            description: "管理论坛分类和板块",
            path: "/categories",
        },
        {label: "系统设置", description: "配置系统参数和选项", path: "/settings"},
    ];

    if (!data) {
        return (
            <Container size="4">
                <Flex justify="center" align="center" style={{height: "50vh"}}>
                    <Text>加载中...</Text>
                </Flex>
            </Container>
        );
    }

    return (
        <Container size="4">
            <Flex direction="column" gap="5">
                {/* 欢迎标题 */}
                <Box>
                    <Heading as="h1" size="6" mb="2">
                        欢迎回来，管理员
                    </Heading>
                    <Text as="p" size="3" color="gray">
                        这里是论坛管理系统后台，您可以在这里管理用户、帖子、分类等。
                    </Text>
                </Box>

                <Separator size="4"/>

                {/* 统计卡片 */}
                <Box>
                    <Heading as="h2" size="4" mb="4">
                        系统概览
                    </Heading>
                    <Grid columns="3" gap="4">
                        {stats.map((stat, index) => (
                            <Card key={index}>
                                <Flex direction="column" gap="2">
                                    <Text size="2" color="gray">
                                        {stat.label}
                                    </Text>
                                    <Text
                                        size="5"
                                        weight="bold"
                                        style={{color: `var(--${stat.color}-11)`}}
                                    >
                                        {stat.value}
                                    </Text>
                                </Flex>
                            </Card>
                        ))}
                    </Grid>
                </Box>

                <Grid columns="1" gap="5">
                    {/* 系统信息 */}
                    <Box>
                        <Heading as="h2" size="4" mb="4">
                            系统信息
                        </Heading>
                        <Card>
                            <Grid columns={{initial: "1", md: "2"}} gapX="6" gapY="3">
                                {systemInfo.map((info, index) => (
                                    <Box key={index} style={{
                                        borderBottom: "1px solid var(--gray-4)",
                                        paddingBottom: "8px"
                                    }}>
                                        <Flex align="center" justify="between">
                                            <Text size="2" weight="medium">
                                                {info.label}
                                            </Text>
                                            <Text size="2" color="gray">
                                                {info.value}
                                            </Text>
                                        </Flex>
                                    </Box>
                                ))}
                            </Grid>
                        </Card>
                    </Box>
                </Grid>

                {/* 快速操作 */}
                <Box>
                    <Heading as="h2" size="4" mb="4">
                        快速操作
                    </Heading>
                    <Card>
                        <Grid columns="4" gap="4">
                            {quickActions.map((action, index) => (
                                <Box
                                    key={index}
                                    style={{
                                        padding: "16px",
                                        backgroundColor: "var(--gray-1)",
                                        borderRadius: "6px",
                                        border: "1px solid var(--gray-4)",
                                        cursor: "pointer",
                                    }}
                                    onClick={() => navigate(action.path)}
                                    onMouseEnter={(e) => {
                                        e.currentTarget.style.backgroundColor = "var(--gray-2)";
                                    }}
                                    onMouseLeave={(e) => {
                                        e.currentTarget.style.backgroundColor = "var(--gray-1)";
                                    }}
                                >
                                    <Flex direction="column" gap="2">
                                        <Text size="3" weight="bold">
                                            {action.label}
                                        </Text>
                                        <Text size="1" color="gray">
                                            {action.description}
                                        </Text>
                                    </Flex>
                                </Box>
                            ))}
                        </Grid>
                    </Card>
                </Box>
            </Flex>
        </Container>
    );
};

export default HomePage;
