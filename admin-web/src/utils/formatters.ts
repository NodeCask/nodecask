// 格式化工具函数

/**
 * 格式化日期时间
 * @param date 日期对象或日期字符串
 * @param format 格式字符串，支持 YYYY, MM, DD, HH, mm, ss
 * @returns 格式化后的日期字符串
 */
export function formatDate(
  date: Date | string,
  format: string = "YYYY-MM-DD",
): string {
  const d = typeof date === "string" ? new Date(date) : date;

  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  const hours = String(d.getHours()).padStart(2, "0");
  const minutes = String(d.getMinutes()).padStart(2, "0");
  const seconds = String(d.getSeconds()).padStart(2, "0");

  return format
    .replace("YYYY", String(year))
    .replace("MM", month)
    .replace("DD", day)
    .replace("HH", hours)
    .replace("mm", minutes)
    .replace("ss", seconds);
}

/**
 * 相对时间格式化（如：5分钟前）
 * @param date 日期对象或日期字符串
 * @returns 相对时间字符串
 */
export function timeAgo(date: Date | string): string {
  const now = new Date();
  const d = typeof date === "string" ? new Date(date) : date;
  const diff = now.getTime() - d.getTime();

  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  const months = Math.floor(days / 30);
  const years = Math.floor(months / 12);

  if (years > 0) return `${years}年前`;
  if (months > 0) return `${months}个月前`;
  if (days > 0) return `${days}天前`;
  if (hours > 0) return `${hours}小时前`;
  if (minutes > 0) return `${minutes}分钟前`;
  return `${seconds}秒前`;
}

/**
 * 格式化数字（如：1.2K, such as 1.2M）
 * @param num 数字
 * @returns 格式化后的数字字符串
 */
export function formatNumber(num: number): string {
  if (num >= 1000000) return (num / 1000000).toFixed(1) + "M";
  if (num >= 1000) return (num / 1000).toFixed(1) + "K";
  return num.toString();
}

/**
 * 格式化文件大小
 * @param bytes 字节数
 * @param decimals 小数位数
 * @returns 格式化后的文件大小字符串
 */
export function formatFileSize(bytes: number, decimals: number = 2): string {
  if (bytes === 0) return "0 Bytes";

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + " " + sizes[i];
}

/**
 * 格式化货币
 * @param amount 金额
 * @param currency 货币符号，默认为人民币
 * @returns 格式化后的货币字符串
 */
export function formatCurrency(amount: number, currency: string = "¥"): string {
  return `${currency}${amount.toFixed(2)}`;
}

/**
 * 截断字符串并添加省略号
 * @param str 原始字符串
 * @param length 最大长度
 * @param suffix 后缀，默认为省略号
 * @returns 截断后的字符串
 */
export function truncate(
  str: string,
  length: number,
  suffix: string = "...",
): string {
  if (str.length <= length) return str;
  return str.substring(0, length) + suffix;
}

/**
 * 首字母大写
 * @param str 字符串
 * @returns 首字母大写的字符串
 */
export function capitalize(str: string): string {
  if (!str) return "";
  return str.charAt(0).toUpperCase() + str.slice(1);
}

/**
 * 格式化百分比
 * @param value 值
 * @param total 总值
 * @param decimals 小数位数
 * @returns 百分比字符串
 */
export function formatPercentage(
  value: number,
  total: number,
  decimals: number = 2,
): string {
  if (total === 0) return "0%";
  const percentage = (value / total) * 100;
  return `${percentage.toFixed(decimals)}%`;
}

/**
 * 格式化时长（秒转时分秒）
 * @param seconds 秒数
 * @returns 格式化后的时长字符串
 */
export function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }
  return `${minutes}:${secs.toString().padStart(2, "0")}`;
}

/**
 * 格式化手机号码（如：138 0013 8000）
 * @param phone 手机号码
 * @returns 格式化后的手机号码
 */
export function formatPhone(phone: string): string {
  if (!phone || phone.length !== 11) return phone;
  return `${phone.substring(0, 3)} ${phone.substring(3, 7)} ${phone.substring(7)}`;
}

/**
 * 格式化身份证号码（如：1101********001X）
 * @param idCard 身份证号码
 * @returns 格式化后的身份证号码
 */
export function formatIdCard(idCard: string): string {
  if (!idCard || idCard.length < 14) return idCard;
  return `${idCard.substring(0, 6)}${"*".repeat(idCard.length - 10)}${idCard.substring(idCard.length - 4)}`;
}
