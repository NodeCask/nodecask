// 会员登录奖励计算脚本
// ----------------------------------------------------------------
// 辅助函数：限制数值范围
function clamp(num, min, max) {
    return Math.min(Math.max(num, min), max);
}
/**
 * 计算登录奖励 (AccessDaysData)
 * 策略：登录属于被动行为，奖励增长较缓。
 */
function calculateLoginRewards(data) {
    var days = data.ContinuousAccessDays || 1;
    // --- 金币计算 (1~10, Max 20) ---
    // 基础逻辑：第1天1金币，之后每多连续1天+1金币，常规封顶10金币
    var coins = Math.min(10, 1 + Math.floor(days / 2));
    // 里程碑奖励：如果是7的倍数天（周循环），直接给最大值 20
    if (days % 7 === 0) {
        coins = 20;
    }
    // --- 信誉分计算 (1~5, Max 10) ---
    // 基础逻辑：前3天给1分，之后缓慢增加，常规封顶5分
    var credit = 1;
    if (days >= 3)
        credit = 2;
    if (days >= 7)
        credit = 3;
    if (days >= 15)
        credit = 5;
    // 里程碑奖励：如果是30的倍数天（月循环），直接给最大值 10
    if (days % 30 === 0) {
        credit = 10;
    }
    return {
        credit: clamp(credit, 1, 10),
        coins: clamp(coins, 1, 20),
    };
}
/**
 * 计算打卡奖励 (CheckInData)
 * 策略：打卡属于主动行为，奖励增长较快，鼓励保持习惯。
 */
function calculateCheckInRewards(data) {
    var days = data.current_continuous_checkin_count || 1;
    // --- 金币计算 (1~10, Max 20) ---
    // 基础逻辑：直接等于连续天数，第1天1个，第10天10个。常规封顶10。
    var coins = Math.min(10, days);
    // 里程碑奖励：
    // 1. 每连续打卡满 7 天 (周奖励)，给 15 金币
    // 2. 每连续打卡满 14/21/28... (倍数奖励)，给 20 金币
    if (days % 7 === 0) {
        coins = (days % 14 === 0) ? 20 : 15;
    }
    // --- 信誉分计算 (1~5, Max 10) ---
    // 基础逻辑：线性增长，每2天加1分。公式：ceil(days / 2)。常规封顶5。
    var credit = Math.min(5, Math.ceil(days / 2));
    // 里程碑奖励：连续打卡超过 21 天，每逢 7 的倍数，给予最大信誉奖励 10
    if (days >= 21 && days % 7 === 0) {
        credit = 10;
    }
    return {
        credit: clamp(credit, 1, 10),
        coins: clamp(coins, 1, 20),
    };
}
