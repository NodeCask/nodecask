import React from "react";
import {Box, Container, Flex, Heading, Tabs, Text,} from "@radix-ui/themes";
import {SiteInfoSettings} from "./settings/SiteInfoSettings";
import {SmtpSettings} from "./settings/SmtpSettings";
import {RegisterSettings} from "./settings/RegisterSettings";
import {PostSettings} from "./settings/PostSettings";
import {LinkFilterSettings} from "./settings/LinkFilterSettings";
import {InjectionSettings} from "./settings/InjectionSettings";
import {LoginRewardsSettings} from "./settings/LoginRewardsSettings";
import {ModeratorSettings} from "./settings/ModeratorSettings";
import {VisitSettings} from "./settings/VisitSettings";
import {TurnstileSettings} from "./settings/TurnstileSettings";
import {AdvancedSettings} from "./settings/AdvancedSettings";

const SettingsPage: React.FC = () => {

    return (
        <Container size="4">
            <Flex direction="column" gap="5">
                {/* Header */}
                <Box>
                    <Heading as="h1" size="6" mb="2">
                        系统设置
                    </Heading>
                    <Text as="p" size="3" color="gray">
                        配置系统参数和选项
                    </Text>
                </Box>

                <Tabs.Root defaultValue="general">
                    <Tabs.List>
                        <Tabs.Trigger value="general">常规设置</Tabs.Trigger>
                        <Tabs.Trigger value="visit">浏览设定</Tabs.Trigger>
                        <Tabs.Trigger value="register">注册设置</Tabs.Trigger>
                        <Tabs.Trigger value="post">发帖设置</Tabs.Trigger>
                        <Tabs.Trigger value="link_filter">链接过滤</Tabs.Trigger>
                        <Tabs.Trigger value="moderator">内容审核</Tabs.Trigger>
                        <Tabs.Trigger value="injection">代码注入</Tabs.Trigger>
                        <Tabs.Trigger value="login_rewards">签到奖励</Tabs.Trigger>
                        <Tabs.Trigger value="email">邮件设置</Tabs.Trigger>
                        <Tabs.Trigger value="turnstile">Turnstile</Tabs.Trigger>
                        <Tabs.Trigger value="advanced">高级配置</Tabs.Trigger>
                    </Tabs.List>

                    <Box pt="4">
                        {/* General Settings Tab */}
                        <Tabs.Content value="general">
                            <SiteInfoSettings/>
                        </Tabs.Content>

                        {/* Visit Settings Tab */}
                        <Tabs.Content value="visit">
                            <VisitSettings/>
                        </Tabs.Content>

                        {/* Register Settings Tab */}
                        <Tabs.Content value="register">
                            <RegisterSettings/>
                        </Tabs.Content>

                        {/* Post Settings Tab */}
                        <Tabs.Content value="post">
                            <PostSettings/>
                        </Tabs.Content>

                        {/* Link Filter Settings Tab */}
                        <Tabs.Content value="link_filter">
                            <LinkFilterSettings/>
                        </Tabs.Content>

                        {/* Moderator Settings Tab */}
                        <Tabs.Content value="moderator">
                            <ModeratorSettings/>
                        </Tabs.Content>

                        {/* Injection Settings Tab */}
                        <Tabs.Content value="injection">
                            <InjectionSettings/>
                        </Tabs.Content>

                        {/* Login Rewards Settings Tab */}
                        <Tabs.Content value="login_rewards">
                            <LoginRewardsSettings/>
                        </Tabs.Content>

                        {/* Email Settings Tab */}
                        <Tabs.Content value="email">
                            <SmtpSettings/>
                        </Tabs.Content>

                        {/* Turnstile Settings Tab */}
                        <Tabs.Content value="turnstile">
                            <TurnstileSettings/>
                        </Tabs.Content>

                        {/* Advanced Settings Tab */}
                        <Tabs.Content value="advanced">
                            <AdvancedSettings/>
                        </Tabs.Content>
                    </Box>
                </Tabs.Root>

            </Flex>
        </Container>
    );
};

export default SettingsPage;
