import React from "react";
import { Button, Flex, Text, Select } from "@radix-ui/themes";

interface PaginationProps {
    currentPage: number;
    totalPages: number;
    onPageChange: (page: number) => void;
    itemsPerPage?: number;
    onItemsPerPageChange?: (items: number) => void;
}

const Pagination: React.FC<PaginationProps> = ({
    currentPage,
    totalPages,
    onPageChange,
    itemsPerPage = 10,
    onItemsPerPageChange,
}) => {
    const handlePrevious = () => {
        if (currentPage > 1) {
            onPageChange(currentPage - 1);
        }
    };

    const handleNext = () => {
        if (currentPage < totalPages) {
            onPageChange(currentPage + 1);
        }
    };

    const handlePageSelect = (page: number) => {
        if (page >= 1 && page <= totalPages) {
            onPageChange(page);
        }
    };

    // Generate page numbers to display
    const getPageNumbers = () => {
        const pages: (number | string)[] = [];
        const maxVisible = 5;

        if (totalPages <= maxVisible) {
            // Show all pages if total is small
            for (let i = 1; i <= totalPages; i++) {
                pages.push(i);
            }
        } else {
            // Show first page
            pages.push(1);

            if (currentPage > 3) {
                pages.push("...");
            }

            // Show pages around current page
            const start = Math.max(2, currentPage - 1);
            const end = Math.min(totalPages - 1, currentPage + 1);

            for (let i = start; i <= end; i++) {
                pages.push(i);
            }

            if (currentPage < totalPages - 2) {
                pages.push("...");
            }

            // Show last page
            pages.push(totalPages);
        }

        return pages;
    };

    return (
        <Flex align="center" justify="between" gap="4" wrap="wrap">
            <Flex align="center" gap="2">
                <Text size="2" color="gray">
                    第 {currentPage} / {totalPages} 页
                </Text>
            </Flex>

            <Flex align="center" gap="2">
                <Button
                    variant="soft"
                    size="2"
                    onClick={handlePrevious}
                    disabled={currentPage === 1}
                >
                    上一页
                </Button>

                {getPageNumbers().map((page, index) => {
                    if (page === "...") {
                        return (
                            <Text key={`ellipsis-${index}`} size="2" color="gray">
                                ...
                            </Text>
                        );
                    }

                    const pageNum = page as number;
                    const isActive = pageNum === currentPage;

                    return (
                        <Button
                            key={pageNum}
                            variant={isActive ? "solid" : "soft"}
                            size="2"
                            onClick={() => handlePageSelect(pageNum)}
                            style={{
                                minWidth: "36px",
                                cursor: isActive ? "default" : "pointer",
                            }}
                        >
                            {pageNum}
                        </Button>
                    );
                })}

                <Button
                    variant="soft"
                    size="2"
                    onClick={handleNext}
                    disabled={currentPage === totalPages}
                >
                    下一页
                </Button>
            </Flex>

            {onItemsPerPageChange && (
                <Flex align="center" gap="2">
                    <Text size="2" color="gray">
                        每页显示
                    </Text>
                    <Select.Root
                        value={itemsPerPage.toString()}
                        onValueChange={(value) => onItemsPerPageChange(Number(value))}
                    >
                        <Select.Trigger />
                        <Select.Content>
                            <Select.Item value="10">10</Select.Item>
                            <Select.Item value="20">20</Select.Item>
                            <Select.Item value="50">50</Select.Item>
                            <Select.Item value="100">100</Select.Item>
                        </Select.Content>
                    </Select.Root>
                    <Text size="2" color="gray">
                        条
                    </Text>
                </Flex>
            )}
        </Flex>
    );
};

export default Pagination;
