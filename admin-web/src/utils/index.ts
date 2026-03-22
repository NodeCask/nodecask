// 工具函数索引文件

// 重新导出格式化工具
import {
  formatDate,
  timeAgo,
  formatNumber,
  formatCurrency,
  truncate,
  capitalize,
} from "./formatters";

// 重新导出验证工具
import {
  isValidEmail,
  isValidPhone,
  isValidUrl,
  isNotEmpty,
} from "./validators";

// 其他工具函数
export const debounce = <T extends (...args: any[]) => any>(
  func: T,
  wait: number,
): ((...args: Parameters<T>) => void) => {
  let timeout: ReturnType<typeof setTimeout>;

  return (...args: Parameters<T>) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => func(...args), wait);
  };
};

export const throttle = <T extends (...args: any[]) => any>(
  func: T,
  limit: number,
): ((...args: Parameters<T>) => void) => {
  let inThrottle: boolean;

  return (...args: Parameters<T>) => {
    if (!inThrottle) {
      func(...args);
      inThrottle = true;
      setTimeout(() => {
        inThrottle = false;
      }, limit);
    }
  };
};

// 本地存储工具
export const storage = {
  get: <T>(key: string, defaultValue?: T): T | null => {
    try {
      const item = localStorage.getItem(key);
      return item ? JSON.parse(item) : defaultValue || null;
    } catch {
      return defaultValue || null;
    }
  },

  set: (key: string, value: any): void => {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch (error) {
      console.error("LocalStorage 存储失败:", error);
    }
  },

  remove: (key: string): void => {
    localStorage.removeItem(key);
  },

  clear: (): void => {
    localStorage.clear();
  },
};

// 日期时间工具
export const dateUtils = {
  formatDate: formatDate,
  timeAgo: timeAgo,
};

// 字符串工具
export const stringUtils = {
  truncate: truncate,
  capitalize: capitalize,

  generateId: (): string => {
    return Math.random().toString(36).substring(2) + Date.now().toString(36);
  },
};

// 数字工具
export const numberUtils = {
  formatNumber: formatNumber,
  formatCurrency: formatCurrency,
};

// 验证工具
export const validation = {
  isEmail: isValidEmail,
  isPhone: isValidPhone,
  isUrl: isValidUrl,
  isNotEmpty: isNotEmpty,
};

// 导出原始函数
export {
  formatDate,
  timeAgo,
  formatNumber,
  formatCurrency,
  truncate,
  capitalize,
  isValidEmail,
  isValidPhone,
  isValidUrl,
  isNotEmpty,
};
