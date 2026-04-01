import React, { useState } from "react";
import { Box, Button, Dialog, Flex, Text, TextField } from "@radix-ui/themes";

export interface UserAddForm {
    email: string;
    username: string;
    password: string;
}

interface UserAddDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onConfirm: (formData: UserAddForm) => Promise<void>;
}

const UserAddDialog: React.FC<UserAddDialogProps> = ({
    open,
    onOpenChange,
    onConfirm,
}) => {
    const [formData, setFormData] = useState<UserAddForm>({
        email: "",
        username: "",
        password: "",
    });
    const [loading, setLoading] = useState(false);

    const handleChange = (field: keyof UserAddForm, value: string) => {
        setFormData((prev) => ({ ...prev, [field]: value }));
    };

    const handleConfirm = async () => {
        if (!formData.email || !formData.username || !formData.password) {
            alert("请填写所有必填字段");
            return;
        }
        setLoading(true);
        try {
            await onConfirm(formData);
            // reset form on success
            setFormData({ email: "", username: "", password: "" });
        } finally {
            setLoading(false);
        }
    };

    // Also handle dialog close to reset state if needed, but keeping it simple here
    const handleOpenChange = (isOpen: boolean) => {
        if (!isOpen) {
            setFormData({ email: "", username: "", password: "" });
        }
        onOpenChange(isOpen);
    };

    return (
        <Dialog.Root open={open} onOpenChange={handleOpenChange}>
            <Dialog.Content style={{ maxWidth: 450 }}>
                <Dialog.Title>添加用户</Dialog.Title>
                <Dialog.Description size="2" mb="4">
                    手动添加一个新用户。
                </Dialog.Description>

                <Flex direction="column" gap="3">
                    <Box>
                        <Text as="label" size="2" weight="medium" mb="1">
                            邮箱地址
                        </Text>
                        <TextField.Root
                            type="email"
                            placeholder="user@example.com"
                            value={formData.email}
                            onChange={(e) => handleChange("email", e.target.value)}
                        />
                    </Box>

                    <Box>
                        <Text as="label" size="2" weight="medium" mb="1">
                            用户名称
                        </Text>
                        <TextField.Root
                            placeholder="username"
                            value={formData.username}
                            onChange={(e) => handleChange("username", e.target.value)}
                        />
                    </Box>

                    <Box>
                        <Text as="label" size="2" weight="medium" mb="1">
                            用户密码
                        </Text>
                        <TextField.Root
                            type="password"
                            placeholder="********"
                            value={formData.password}
                            onChange={(e) => handleChange("password", e.target.value)}
                        />
                    </Box>
                </Flex>

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray" disabled={loading}>
                            取消
                        </Button>
                    </Dialog.Close>
                    <Button onClick={handleConfirm} disabled={loading}>
                        {loading ? "添加中..." : "确定"}
                    </Button>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};

export default UserAddDialog;
