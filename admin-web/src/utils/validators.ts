// 验证工具函数

/**
 * 验证电子邮件格式
 * @param email 电子邮件地址
 * @returns 是否为有效的电子邮件格式
 */
export function isValidEmail(email: string): boolean {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email);
}

/**
 * 验证手机号码格式（中国大陆）
 * @param phone 手机号码
 * @returns 是否为有效的手机号码格式
 */
export function isValidPhone(phone: string): boolean {
  const phoneRegex = /^1[3-9]\d{9}$/;
  return phoneRegex.test(phone);
}

/**
 * 验证URL格式
 * @param url URL地址
 * @returns 是否为有效的URL格式
 */
export function isValidUrl(url: string): boolean {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
}

/**
 * 验证密码强度
 * @param password 密码
 * @param options 验证选项
 * @returns 验证结果
 */
export function validatePassword(
  password: string,
  options: {
    minLength?: number;
    requireUppercase?: boolean;
    requireLowercase?: boolean;
    requireNumbers?: boolean;
    requireSpecialChars?: boolean;
  } = {},
): { isValid: boolean; errors: string[] } {
  const {
    minLength = 8,
    requireUppercase = true,
    requireLowercase = true,
    requireNumbers = true,
    requireSpecialChars = true,
  } = options;

  const errors: string[] = [];

  if (password.length < minLength) {
    errors.push(`密码长度不能少于 ${minLength} 个字符`);
  }

  if (requireUppercase && !/[A-Z]/.test(password)) {
    errors.push("密码必须包含至少一个大写字母");
  }

  if (requireLowercase && !/[a-z]/.test(password)) {
    errors.push("密码必须包含至少一个小写字母");
  }

  if (requireNumbers && !/\d/.test(password)) {
    errors.push("密码必须包含至少一个数字");
  }

  if (requireSpecialChars && !/[!@#$%^&*(),.?":{}|<>]/.test(password)) {
    errors.push("密码必须包含至少一个特殊字符");
  }

  return {
    isValid: errors.length === 0,
    errors,
  };
}

/**
 * 验证用户名格式
 * @param username 用户名
 * @param options 验证选项
 * @returns 验证结果
 */
export function validateUsername(
  username: string,
  options: {
    minLength?: number;
    maxLength?: number;
    allowSpecialChars?: boolean;
  } = {},
): { isValid: boolean; errors: string[] } {
  const { minLength = 3, maxLength = 20, allowSpecialChars = false } = options;

  const errors: string[] = [];

  if (username.length < minLength) {
    errors.push(`用户名长度不能少于 ${minLength} 个字符`);
  }

  if (username.length > maxLength) {
    errors.push(`用户名长度不能超过 ${maxLength} 个字符`);
  }

  // 用户名只能包含字母、数字、下划线
  if (!allowSpecialChars && !/^[a-zA-Z0-9_]+$/.test(username)) {
    errors.push("用户名只能包含字母、数字和下划线");
  }

  // 用户名不能以数字开头
  if (/^\d/.test(username)) {
    errors.push("用户名不能以数字开头");
  }

  return {
    isValid: errors.length === 0,
    errors,
  };
}

/**
 * 验证身份证号码格式（中国大陆）
 * @param idCard 身份证号码
 * @returns 是否为有效的身份证号码格式
 */
export function isValidIdCard(idCard: string): boolean {
  // 简单验证：15位或18位
  const regex = /(^\d{15}$)|(^\d{17}(\d|X|x)$)/;
  return regex.test(idCard);
}

/**
 * 验证是否为空值
 * @param value 值
 * @returns 是否不为空
 */
export function isNotEmpty(value: any): boolean {
  if (typeof value === "string") return value.trim().length > 0;
  if (Array.isArray(value)) return value.length > 0;
  if (value === null || value === undefined) return false;
  return true;
}

/**
 * 验证数字范围
 * @param value 数值
 * @param min 最小值
 * @param max 最大值
 * @returns 是否在范围内
 */
export function isInRange(value: number, min: number, max: number): boolean {
  return value >= min && value <= max;
}

/**
 * 验证文件类型
 * @param file 文件对象或文件名
 * @param allowedTypes 允许的文件类型数组
 * @returns 是否为允许的文件类型
 */
export function isValidFileType(
  file: File | string,
  allowedTypes: string[],
): boolean {
  const fileName = typeof file === "string" ? file : file.name;
  const extension = fileName.split(".").pop()?.toLowerCase() || "";

  return allowedTypes.some((type) => {
    const normalizedType = type.toLowerCase().replace(".", "");
    return extension === normalizedType;
  });
}

/**
 * 验证文件大小
 * @param file 文件对象
 * @param maxSizeMB 最大文件大小（MB）
 * @returns 文件大小是否在限制内
 */
export function isValidFileSize(file: File, maxSizeMB: number): boolean {
  const maxSizeBytes = maxSizeMB * 1024 * 1024;
  return file.size <= maxSizeBytes;
}

/**
 * 验证日期格式
 * @param dateString 日期字符串
 * @param format 日期格式，支持：YYYY-MM-DD, DD/MM/YYYY, MM/DD/YYYY
 * @returns 是否为有效的日期格式
 */
export function isValidDate(
  dateString: string,
  format: "YYYY-MM-DD" | "DD/MM/YYYY" | "MM/DD/YYYY" = "YYYY-MM-DD",
): boolean {
  let regex: RegExp;

  switch (format) {
    case "YYYY-MM-DD":
      regex = /^\d{4}-\d{2}-\d{2}$/;
      break;
    case "DD/MM/YYYY":
      regex = /^\d{2}\/\d{2}\/\d{4}$/;
      break;
    case "MM/DD/YYYY":
      regex = /^\d{2}\/\d{2}\/\d{4}$/;
      break;
    default:
      return false;
  }

  if (!regex.test(dateString)) {
    return false;
  }

  const date = new Date(dateString);
  return !isNaN(date.getTime());
}

/**
 * 验证是否为数字
 * @param value 值
 * @returns 是否为数字
 */
export function isNumeric(value: any): boolean {
  if (typeof value === "number") return true;
  if (typeof value !== "string") return false;
  return !isNaN(parseFloat(value)) && isFinite(Number(value));
}

/**
 * 验证数组是否包含特定值
 * @param array 数组
 * @param value 值
 * @returns 是否包含
 */
export function contains<T>(array: T[], value: T): boolean {
  return array.includes(value);
}

/**
 * 验证字符串长度
 * @param str 字符串
 * @param min 最小长度
 * @param max 最大长度
 * @returns 长度是否在范围内
 */
export function isValidLength(str: string, min: number, max: number): boolean {
  const length = str.length;
  return length >= min && length <= max;
}
