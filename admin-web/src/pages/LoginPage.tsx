import React, {useState} from "react";
import {useLocation, useNavigate} from "react-router";
import {Box, Button, Card, Container, Flex, Heading, Text, TextField,} from "@radix-ui/themes";
import {login, verifyTotp, SysError} from "../api";

const LoginPage: React.FC = () => {
    const navigate = useNavigate();
    const location = useLocation();
    
    // Login State
    const [step, setStep] = useState<'credentials' | 'totp'>('credentials');
    
    // Form Data
    const [username, setUsername] = useState("");
    const [password, setPassword] = useState("");
    const [totpCode, setTotpCode] = useState("");
    
    // UI State
    const [isLoading, setIsLoading] = useState(false);
    const [formError, setFormError] = useState<string | null>(null);

    // 从路由状态获取重定向来源
    const from = location.state?.from || "/";

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();

        // 重置错误状态
        setFormError(null);

        if (step === 'credentials') {
            // 凭证验证
            if (!username.trim()) {
                setFormError("请输入用户名");
                return;
            }

            if (!password.trim()) {
                setFormError("请输入密码");
                return;
            }

            setIsLoading(true);
            try {
                // 调用登录函数
                const resp = await login(username, password);
                
                if (resp.totp) {
                    setStep('totp');
                } else {
                    // 登录成功，跳转到来源页面或首页
                    navigate(from, {replace: true});
                }
            } catch (err) {
                console.log(err);
                if(err instanceof SysError) {
                    setFormError(err.message);
                }else{
                    setFormError("登录失败，请稍后重试");
                }
            } finally {
                setIsLoading(false);
            }
        } else {
            // TOTP 验证
            if (!totpCode.trim()) {
                setFormError("请输入验证码");
                return;
            }
            
            setIsLoading(true);
            try {
                await verifyTotp(username, totpCode);
                navigate(from, {replace: true});
            } catch (err) {
                 console.log(err);
                if(err instanceof SysError) {
                    setFormError(err.message);
                }else{
                    setFormError("验证失败，请稍后重试");
                }
            } finally {
                setIsLoading(false);
            }
        }
    };

    return (
        <Container size="1" style={{height: "100vh", paddingTop: "80px"}}>
            <Card style={{maxWidth: "400px", margin: "0 auto"}}>
                <Flex direction="column" gap="4">
                    <Box>
                        <Heading as="h1" size="5" align="center">
                            管理员登录
                        </Heading>
                        <Text as="p" size="2" color="gray" align="center" mt="2">
                            {step === 'credentials' ? "请输入管理员账号和密码" : "请输入两步验证码"}
                        </Text>
                    </Box>

                    {(formError) && (
                        <Box
                            style={{
                                padding: "12px",
                                backgroundColor: "var(--red-3)",
                                borderRadius: "4px",
                                border: "1px solid var(--red-5)",
                            }}
                        >
                            <Text size="2" color="red">
                                {formError}
                            </Text>
                        </Box>
                    )}

                    <form onSubmit={handleSubmit}>
                        <Flex direction="column" gap="3">
                            {step === 'credentials' ? (
                                <>
                                    <Box>
                                        <Text as="label" size="2" weight="medium" htmlFor="username">
                                            用户名
                                        </Text>
                                        <TextField.Root
                                            id="username"
                                            value={username}
                                            onChange={(e) => setUsername(e.target.value)}
                                            placeholder="请输入用户名"
                                            required
                                            disabled={isLoading}
                                            autoComplete="username"
                                            autoFocus
                                            mt="2"
                                        />
                                    </Box>

                                    <Box>
                                        <Text as="label" size="2" weight="medium" htmlFor="password">
                                            密码
                                        </Text>
                                        <TextField.Root
                                            id="password"
                                            type="password"
                                            value={password}
                                            onChange={(e) => setPassword(e.target.value)}
                                            placeholder="请输入密码"
                                            required
                                            disabled={isLoading}
                                            mt="2"
                                        />
                                    </Box>
                                </>
                            ) : (
                                <Box>
                                    <Text as="label" size="2" weight="medium" htmlFor="totp">
                                        验证码 (TOTP)
                                    </Text>
                                    <TextField.Root
                                        id="totp"
                                        value={totpCode}
                                        onChange={(e) => setTotpCode(e.target.value)}
                                        placeholder="6位数字验证码"
                                        required
                                        disabled={isLoading}
                                        autoFocus
                                        autoComplete="one-time-code"
                                        mt="2"
                                    />
                                </Box>
                            )}

                            <Button
                                type="submit"
                                size="3"
                                disabled={isLoading}
                                style={{width: "100%", marginTop: "8px"}}
                            >
                                {isLoading ? "验证中..." : (step === 'credentials' ? "登录" : "验证")}
                            </Button>
                            
                            {step === 'totp' && (
                                <Button 
                                    type="button" 
                                    variant="soft" 
                                    color="gray" 
                                    onClick={() => {
                                        setStep('credentials');
                                        setTotpCode("");
                                        setFormError(null);
                                    }}
                                >
                                    返回登录
                                </Button>
                            )}
                        </Flex>
                    </form>
                </Flex>
            </Card>
        </Container>
    );
};

export default LoginPage;