import React, { useEffect, useState, useCallback, useRef } from "react";
import {
    Box,
    Button,
    Card,
    Dialog,
    Flex,
    Heading,
    IconButton,
    Table,
    Text,
    TextField,
    DropdownMenu,
    Spinner,
    Badge,
} from "@radix-ui/themes";
import {
    FileIcon,
    ChevronLeftIcon,
    ChevronRightIcon,
    DotsHorizontalIcon,
    TrashIcon,
    Pencil1Icon,
    UploadIcon,
    DownloadIcon,
    HomeIcon,
    ArchiveIcon,
    MoveIcon,
} from "@radix-ui/react-icons";
import { push } from "../api";

// 文件信息接口
interface FileInfo {
    name: string;
    path: string;
    is_dir: boolean;
    size?: number;
    last_modified?: string;
}

// API 函数
async function listFiles(path: string): Promise<FileInfo[]> {
    return push<FileInfo[], { p: string }>("/store/list", { p: path });
}

async function deleteFile(path: string): Promise<void> {
    return push("/store/delete", { p: path });
}

async function renameFile(path: string, newName: string): Promise<void> {
    return push("/store/rename", { p: path, new_name: newName });
}

async function moveFile(path: string, dir: string): Promise<void> {
    return push("/store/move", { p: path, dir: dir });
}

// 格式化文件大小
function formatFileSize(bytes?: number): string {
    if (bytes === undefined || bytes === null) return "-";
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

// 格式化日期
function formatDate(dateStr?: string): string {
    if (!dateStr) return "-";
    const date = new Date(dateStr);
    return date.toLocaleDateString() + " " + date.toLocaleTimeString();
}

// 面包屑组件
const Breadcrumb: React.FC<{
    currentPath: string;
    onNavigate: (path: string) => void;
}> = ({ currentPath, onNavigate }) => {
    const parts = currentPath ? currentPath.split("/").filter(Boolean) : [];

    return (
        <Flex align="center" gap="1" wrap="wrap">
            <IconButton
                variant="ghost"
                size="1"
                onClick={() => onNavigate("")}
                title="根目录"
            >
                <HomeIcon />
            </IconButton>
            {parts.map((part, index) => {
                const path = parts.slice(0, index + 1).join("/");
                return (
                    <React.Fragment key={path}>
                        <ChevronRightIcon />
                        <Button
                            variant="ghost"
                            size="1"
                            onClick={() => onNavigate(path)}
                        >
                            {part}
                        </Button>
                    </React.Fragment>
                );
            })}
        </Flex>
    );
};

// 重命名对话框
const RenameDialog: React.FC<{
    open: boolean;
    onOpenChange: (open: boolean) => void;
    file: FileInfo | null;
    onRename: (newName: string) => void;
}> = ({ open, onOpenChange, file, onRename }) => {
    const [newName, setNewName] = useState("");

    useEffect(() => {
        if (file) {
            setNewName(file.name);
        }
    }, [file]);

    const handleSubmit = () => {
        if (newName && newName !== file?.name) {
            onRename(newName);
        }
        onOpenChange(false);
    };

    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Content style={{ maxWidth: 400 }}>
                <Dialog.Title>重命名</Dialog.Title>
                <Dialog.Description size="2" mb="4">
                    将 "{file?.name}" 重命名为:
                </Dialog.Description>

                <TextField.Root
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    placeholder="输入新名称"
                    onKeyDown={(e) => {
                        if (e.key === "Enter") handleSubmit();
                    }}
                />

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray">
                            取消
                        </Button>
                    </Dialog.Close>
                    <Button onClick={handleSubmit}>确定</Button>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};

// 移动对话框
const MoveDialog: React.FC<{
    open: boolean;
    onOpenChange: (open: boolean) => void;
    file: FileInfo | null;
    onMove: (targetDir: string) => void;
}> = ({ open, onOpenChange, file, onMove }) => {
    const [targetDir, setTargetDir] = useState("");

    const handleSubmit = () => {
        onMove(targetDir);
        setTargetDir("");
        onOpenChange(false);
    };

    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Content style={{ maxWidth: 400 }}>
                <Dialog.Title>移动文件</Dialog.Title>
                <Dialog.Description size="2" mb="4">
                    将 "{file?.name}" 移动到:
                </Dialog.Description>

                <TextField.Root
                    value={targetDir}
                    onChange={(e) => setTargetDir(e.target.value)}
                    placeholder="输入目标目录路径（留空表示根目录）"
                    onKeyDown={(e) => {
                        if (e.key === "Enter") handleSubmit();
                    }}
                />

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray">
                            取消
                        </Button>
                    </Dialog.Close>
                    <Button onClick={handleSubmit}>移动</Button>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};

// 上传对话框
const UploadDialog: React.FC<{
    open: boolean;
    onOpenChange: (open: boolean) => void;
    currentPath: string;
    onUploadComplete: () => void;
}> = ({ open, onOpenChange, currentPath, onUploadComplete }) => {
    const [uploading, setUploading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const fileInputRef = useRef<HTMLInputElement>(null);

    const handleUpload = async (files: FileList | null) => {
        if (!files || files.length === 0) return;

        setUploading(true);
        setError(null);

        try {
            const formData = new FormData();
            formData.append("path", currentPath);
            for (let i = 0; i < files.length; i++) {
                formData.append("file", files[i]);
            }

            const token = localStorage.getItem("admin_token") || "";
            const response = await fetch("/mod/store/upload", {
                method: "POST",
                headers: {
                    token: token,
                },
                body: formData,
            });

            const result = await response.json();
            if (result.code === 0) {
                onUploadComplete();
                onOpenChange(false);
            } else {
                setError(result.message || "上传失败");
            }
        } catch (err) {
            setError(err instanceof Error ? err.message : "上传失败");
        } finally {
            setUploading(false);
        }
    };

    return (
        <Dialog.Root open={open} onOpenChange={onOpenChange}>
            <Dialog.Content style={{ maxWidth: 400 }}>
                <Dialog.Title>上传文件</Dialog.Title>
                <Dialog.Description size="2" mb="4">
                    上传文件到: {currentPath || "/"}
                </Dialog.Description>

                <Flex direction="column" gap="3">
                    <input
                        ref={fileInputRef}
                        type="file"
                        multiple
                        style={{ display: "none" }}
                        onChange={(e) => handleUpload(e.target.files)}
                    />
                    <Button
                        onClick={() => fileInputRef.current?.click()}
                        disabled={uploading}
                    >
                        {uploading ? (
                            <>
                                <Spinner size="1" /> 上传中...
                            </>
                        ) : (
                            <>
                                <UploadIcon /> 选择文件
                            </>
                        )}
                    </Button>

                    {error && (
                        <Text color="red" size="2">
                            {error}
                        </Text>
                    )}
                </Flex>

                <Flex gap="3" mt="4" justify="end">
                    <Dialog.Close>
                        <Button variant="soft" color="gray" disabled={uploading}>
                            关闭
                        </Button>
                    </Dialog.Close>
                </Flex>
            </Dialog.Content>
        </Dialog.Root>
    );
};

// 主组件
const FileManagerPage: React.FC = () => {
    const [files, setFiles] = useState<FileInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [currentPath, setCurrentPath] = useState("");
    const [pathHistory, setPathHistory] = useState<string[]>([""]);
    const [historyIndex, setHistoryIndex] = useState(0);

    // 对话框状态
    const [renameDialogOpen, setRenameDialogOpen] = useState(false);
    const [moveDialogOpen, setMoveDialogOpen] = useState(false);
    const [uploadDialogOpen, setUploadDialogOpen] = useState(false);
    const [selectedFile, setSelectedFile] = useState<FileInfo | null>(null);

    // 加载文件列表
    const loadFiles = useCallback(async () => {
        try {
            setLoading(true);
            const data = await listFiles(currentPath);
            setFiles(data || []);
        } catch (error) {
            console.error("Failed to load files:", error);
            setFiles([]);
        } finally {
            setLoading(false);
        }
    }, [currentPath]);

    useEffect(() => {
        loadFiles();
    }, [loadFiles]);

    // 导航到目录
    const navigateTo = (path: string) => {
        const newHistory = pathHistory.slice(0, historyIndex + 1);
        newHistory.push(path);
        setPathHistory(newHistory);
        setHistoryIndex(newHistory.length - 1);
        setCurrentPath(path);
    };

    // 后退
    const goBack = () => {
        if (historyIndex > 0) {
            setHistoryIndex(historyIndex - 1);
            setCurrentPath(pathHistory[historyIndex - 1]);
        }
    };

    // 前进
    const goForward = () => {
        if (historyIndex < pathHistory.length - 1) {
            setHistoryIndex(historyIndex + 1);
            setCurrentPath(pathHistory[historyIndex + 1]);
        }
    };

    // 点击文件/文件夹
    const handleItemClick = (file: FileInfo) => {
        if (file.is_dir) {
            navigateTo(file.path.replace(/\/$/, ""));
        }
    };

    // 删除文件
    const handleDelete = async (file: FileInfo) => {
        if (!confirm(`确定要删除 "${file.name}" 吗？`)) return;
        try {
            await deleteFile(file.path);
            loadFiles();
        } catch (error) {
            console.error("Failed to delete:", error);
            alert("删除失败");
        }
    };

    // 重命名
    const handleRename = async (newName: string) => {
        if (!selectedFile) return;
        try {
            await renameFile(selectedFile.path, newName);
            loadFiles();
        } catch (error) {
            console.error("Failed to rename:", error);
            alert("重命名失败");
        }
    };

    // 移动
    const handleMove = async (targetDir: string) => {
        if (!selectedFile) return;
        try {
            await moveFile(selectedFile.path, targetDir);
            loadFiles();
        } catch (error) {
            console.error("Failed to move:", error);
            alert("移动失败");
        }
    };

    // 下载
    const handleDownload = async (file: FileInfo) => {
        try {
            const token = localStorage.getItem("admin_token") || "";
            const response = await fetch("/mod/store/download", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    token: token,
                },
                body: JSON.stringify({ p: file.path }),
            });

            if (!response.ok) {
                throw new Error("下载失败");
            }

            const blob = await response.blob();
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = file.name;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            window.URL.revokeObjectURL(url);
        } catch (error) {
            console.error("Failed to download:", error);
            alert("下载失败");
        }
    };

    return (
        <Box>
            <Flex justify="between" align="center" mb="4">
                <Heading size="6">文件管理</Heading>
                <Button onClick={() => setUploadDialogOpen(true)}>
                    <UploadIcon /> 上传文件
                </Button>
            </Flex>

            {/* 导航栏 */}
            <Card mb="4">
                <Flex align="center" gap="3">
                    <Flex gap="1">
                        <IconButton
                            variant="soft"
                            disabled={historyIndex <= 0}
                            onClick={goBack}
                            title="后退"
                        >
                            <ChevronLeftIcon />
                        </IconButton>
                        <IconButton
                            variant="soft"
                            disabled={historyIndex >= pathHistory.length - 1}
                            onClick={goForward}
                            title="前进"
                        >
                            <ChevronRightIcon />
                        </IconButton>
                    </Flex>
                    <Breadcrumb currentPath={currentPath} onNavigate={navigateTo} />
                </Flex>
            </Card>

            {/* 文件列表 */}
            <Card variant="ghost">
                <Table.Root variant="surface">
                    <Table.Header>
                        <Table.Row>
                            <Table.ColumnHeaderCell style={{ minWidth: "300px" }}>
                                名称
                            </Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>
                                大小
                            </Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{ minWidth: "180px" }}>
                                修改时间
                            </Table.ColumnHeaderCell>
                            <Table.ColumnHeaderCell style={{ minWidth: "100px" }}>
                                操作
                            </Table.ColumnHeaderCell>
                        </Table.Row>
                    </Table.Header>

                    <Table.Body>
                        {loading ? (
                            <Table.Row>
                                <Table.Cell colSpan={4} style={{ textAlign: "center" }}>
                                    <Flex justify="center" align="center" py="4">
                                        <Spinner size="3" />
                                    </Flex>
                                </Table.Cell>
                            </Table.Row>
                        ) : files.length === 0 ? (
                            <Table.Row>
                                <Table.Cell colSpan={4} style={{ textAlign: "center" }}>
                                    <Text color="gray">空文件夹</Text>
                                </Table.Cell>
                            </Table.Row>
                        ) : (
                            files.map((file) => (
                                <Table.Row key={file.path}>
                                    <Table.Cell>
                                        <Flex
                                            align="center"
                                            gap="2"
                                            onClick={() => handleItemClick(file)}
                                            style={{
                                                cursor: file.is_dir ? "pointer" : "default",
                                            }}
                                        >
                                            {file.is_dir ? (
                                                <ArchiveIcon color="orange" />
                                            ) : (
                                                <FileIcon />
                                            )}
                                            <Text
                                                weight={file.is_dir ? "medium" : "regular"}
                                                style={{
                                                    textDecoration: file.is_dir
                                                        ? "none"
                                                        : "none",
                                                }}
                                            >
                                                {file.name}
                                            </Text>
                                            {file.is_dir && (
                                                <Badge color="orange" variant="soft" size="1">
                                                    目录
                                                </Badge>
                                            )}
                                        </Flex>
                                    </Table.Cell>
                                    <Table.Cell>
                                        <Text color="gray">
                                            {file.is_dir ? "-" : formatFileSize(file.size)}
                                        </Text>
                                    </Table.Cell>
                                    <Table.Cell>
                                        <Text color="gray">
                                            {formatDate(file.last_modified)}
                                        </Text>
                                    </Table.Cell>
                                    <Table.Cell>
                                        <DropdownMenu.Root>
                                            <DropdownMenu.Trigger>
                                                <IconButton variant="ghost">
                                                    <DotsHorizontalIcon />
                                                </IconButton>
                                            </DropdownMenu.Trigger>
                                            <DropdownMenu.Content>
                                                {!file.is_dir && (
                                                    <DropdownMenu.Item
                                                        onClick={() => handleDownload(file)}
                                                    >
                                                        <DownloadIcon /> 下载
                                                    </DropdownMenu.Item>
                                                )}
                                                <DropdownMenu.Item
                                                    onClick={() => {
                                                        setSelectedFile(file);
                                                        setRenameDialogOpen(true);
                                                    }}
                                                >
                                                    <Pencil1Icon /> 重命名
                                                </DropdownMenu.Item>
                                                <DropdownMenu.Item
                                                    onClick={() => {
                                                        setSelectedFile(file);
                                                        setMoveDialogOpen(true);
                                                    }}
                                                >
                                                    <MoveIcon /> 移动
                                                </DropdownMenu.Item>
                                                <DropdownMenu.Separator />
                                                <DropdownMenu.Item
                                                    color="red"
                                                    onClick={() => handleDelete(file)}
                                                >
                                                    <TrashIcon /> 删除
                                                </DropdownMenu.Item>
                                            </DropdownMenu.Content>
                                        </DropdownMenu.Root>
                                    </Table.Cell>
                                </Table.Row>
                            ))
                        )}
                    </Table.Body>
                </Table.Root>
            </Card>

            {/* 对话框 */}
            <RenameDialog
                open={renameDialogOpen}
                onOpenChange={setRenameDialogOpen}
                file={selectedFile}
                onRename={handleRename}
            />

            <MoveDialog
                open={moveDialogOpen}
                onOpenChange={setMoveDialogOpen}
                file={selectedFile}
                onMove={handleMove}
            />

            <UploadDialog
                open={uploadDialogOpen}
                onOpenChange={setUploadDialogOpen}
                currentPath={currentPath}
                onUploadComplete={loadFiles}
            />
        </Box>
    );
};

export default FileManagerPage;
