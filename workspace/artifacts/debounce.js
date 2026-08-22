/**
 * debounce.js
 * 防抖函数：在事件被连续触发时，延迟执行回调，并取消前次定时器。
 * 适用于搜索输入、窗口缩放等高频触发场景。
 */

/**
 * 创建防抖函数
 * @param {Function} func - 需要防抖执行的函数
 * @param {number} wait - 延迟执行毫秒数
 * @param {boolean} [immediate=false] - 是否立即执行（首次触发时立即调用，后续等待期内不触发）
 * @returns {Function} 防抖包装后的函数
 */
function debounce(func, wait, immediate = false) {
  // 参数类型校验（最佳实践）
  if (typeof func !== 'function') {
    throw new TypeError('Expected a function as first argument');
  }
  const waitNum = Number(wait);
  if (Number.isNaN(waitNum) || waitNum < 0) {
    throw new TypeError('Expected a non-negative number as wait');
  }

  let timeoutId = null; // 定时器 ID
  let lastArgs = null;  // 保存最后一次调用参数（用于延迟执行）
  let lastThis = null;  // 保存最后一次调用的 this 上下文
  let result;           // 存储函数执行结果（用于 immediate 模式）

  // 清除定时器的内部函数
  const clearTimer = () => {
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
  };

  // 延迟执行的内部函数
  const later = () => {
    timeoutId = null;
    if (!immediate) {
      // 非立即模式：执行原函数
      result = func.apply(lastThis, lastArgs);
      // 清理引用，避免内存泄漏
      lastArgs = lastThis = null;
    }
    // 立即模式在延迟期结束后无需额外操作（已在触发时执行）
  };

  // 返回防抖包装函数
  const debounced = function(...args) {
    // 保存当前上下文和参数
    lastThis = this;
    lastArgs = args;

    // 首次调用且 immediate 为 true 时立即执行
    const callNow = immediate && timeoutId === null;

    // 清除前次定时器（核心防抖逻辑）
    clearTimer();

    // 设置新定时器
    timeoutId = setTimeout(later, waitNum);

    // 立即执行场景
    if (callNow) {
      result = func.apply(lastThis, lastArgs);
      lastArgs = lastThis = null;
    }

    return result;
  };

  // 添加取消方法（最佳实践：允许手动取消）
  debounced.cancel = () => {
    clearTimer();
    lastArgs = lastThis = null;
  };

  // 添加立即执行方法（便于测试或特殊需求）
  debounced.flush = () => {
    if (timeoutId !== null) {
      clearTimer();
      if (!immediate) {
        result = func.apply(lastThis, lastArgs);
        lastArgs = lastThis = null;
      }
    }
    return result;
  };

  return debounced;
}

// 导出（支持 CommonJS 和 ES Module）
if (typeof module !== 'undefined' && module.exports) {
  module.exports = debounce;
} else if (typeof define === 'function' && define.amd) {
  define([], () => debounce);
} else {
  // 浏览器全局挂载
  window.debounce = debounce;
}