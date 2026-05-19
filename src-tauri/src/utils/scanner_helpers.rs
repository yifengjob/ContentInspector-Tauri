/// 扫描器辅助工具模块
/// 
/// 提供环形缓冲区、自适应节流、批量发送等高级功能

use std::time::{Duration, Instant};
use tauri::Emitter; // 导入Emitter trait

/// 环形缓冲区 - O(1)时间复杂度的日志存储
#[allow(dead_code)]
pub struct RingBuffer<T> {
    buffer: Vec<Option<T>>,
    head: usize,
    count: usize,
    capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
    /// 创建新的环形缓冲区
    #[allow(dead_code)]
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![None; capacity],
            head: 0,
            count: 0,
            capacity,
        }
    }

    /// 添加元素（覆盖最旧的元素）
    #[allow(dead_code)]
    pub fn push(&mut self, item: T) {
        let index = self.head % self.capacity;
        self.buffer[index] = Some(item);
        self.head += 1;
        
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// 转换为Vec（按插入顺序）
    #[allow(dead_code)]
    pub fn to_vec(&self) -> Vec<T> {
        if self.count == 0 {
            return vec![];
        }

        let mut result = Vec::with_capacity(self.count);
        
        if self.count < self.capacity {
            // 未满，直接从头开始
            for i in 0..self.count {
                if let Some(item) = &self.buffer[i] {
                    result.push(item.clone());
                }
            }
        } else {
            // 已满，从head位置开始循环读取
            let start = self.head % self.capacity;
            for i in 0..self.capacity {
                let index = (start + i) % self.capacity;
                if let Some(item) = &self.buffer[index] {
                    result.push(item.clone());
                }
            }
        }

        result
    }

    /// 获取元素数量
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.count
    }

    /// 检查是否为空
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// 清空缓冲区
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.buffer.fill(None);
        self.head = 0;
        self.count = 0;
    }
}

/// 自适应进度更新器
#[allow(dead_code)]
pub struct AdaptiveProgressUpdater {
    base_interval: Duration,
    last_update: Instant,
    update_count: u64,
    start_time: Instant,
    total_count: u64,
}

impl AdaptiveProgressUpdater {
    /// 创建新的自适应更新器
    #[allow(dead_code)]
    pub fn new(base_interval_ms: u64) -> Self {
        Self {
            base_interval: Duration::from_millis(base_interval_ms),
            last_update: Instant::now(),
            update_count: 0,
            start_time: Instant::now(),
            total_count: 0,
        }
    }

    /// 检查是否应该更新进度
    #[allow(dead_code)]
    pub fn should_update(&mut self, _current_count: u64, total_count: u64) -> bool {
        self.total_count = total_count;
        let now = Instant::now();
        let elapsed_since_start = now.duration_since(self.start_time);
        let elapsed_since_last = now.duration_since(self.last_update);

        // 初始阶段（3秒内）：快速通过，不节流
        if elapsed_since_start < Duration::from_secs(3) {
            self.last_update = now;
            self.update_count += 1;
            return true;
        }

        // 大量文件场景（>10000）：自动降频
        let effective_interval = if total_count > 10000 {
            self.base_interval * 2  // 降频到2倍间隔
        } else {
            self.base_interval
        };

        // 正常节流
        if elapsed_since_last >= effective_interval {
            self.last_update = now;
            self.update_count += 1;
            return true;
        }

        false
    }

    /// 获取更新次数
    #[allow(dead_code)]
    pub fn update_count(&self) -> u64 {
        self.update_count
    }

    /// 重置计时器
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.last_update = Instant::now();
        self.update_count = 0;
    }
}

/// 批量结果发送器
#[allow(dead_code)]
pub struct ResultBatchSender<T: serde::Serialize + Clone> {
    buffer: Vec<T>,
    batch_size: usize,
    timeout: Duration,
    last_send: Instant,
    event_name: String,
}

impl<T: serde::Serialize + Clone> ResultBatchSender<T> {
    /// 创建新的批量发送器
    #[allow(dead_code)]
    pub fn new(batch_size: usize, timeout_ms: u64, event_name: &str) -> Self {
        Self {
            buffer: Vec::with_capacity(batch_size),
            batch_size,
            timeout: Duration::from_millis(timeout_ms),
            last_send: Instant::now(),
            event_name: event_name.to_string(),
        }
    }

    /// 添加结果到缓冲区
    #[allow(dead_code)]
    pub async fn add_result(
        &mut self,
        result: T,
        app_handle: &tauri::AppHandle,
    ) {
        self.buffer.push(result);

        // 检查是否应该发送
        let should_send = self.buffer.len() >= self.batch_size
            || self.last_send.elapsed() >= self.timeout;

        if should_send && !self.buffer.is_empty() {
            self.flush(app_handle).await;
        }
    }

    /// 强制发送所有缓冲的结果
    #[allow(dead_code)]
    pub async fn flush(&mut self, app_handle: &tauri::AppHandle) {
        if self.buffer.is_empty() {
            return;
        }

        // 批量发送
        for result in self.buffer.drain(..) {
            let _ = app_handle.emit(&self.event_name, result);
        }

        self.last_send = Instant::now();
    }

    /// 获取缓冲区大小
    #[allow(dead_code)]
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

/// 日志抑制器（数量+时间双重触发）
#[allow(dead_code)]
pub struct LogThrottler {
    count_interval: u64,
    time_interval: Duration,
    last_log_time: Option<Instant>,
    suppressed_count: u64,
}

impl LogThrottler {
    /// 创建新的日志抑制器
    #[allow(dead_code)]
    pub fn new(count_interval: u64, time_interval_ms: u64) -> Self {
        Self {
            count_interval,
            time_interval: Duration::from_millis(time_interval_ms),
            last_log_time: None,
            suppressed_count: 0,
        }
    }

    /// 检查是否应该输出日志
    #[allow(dead_code)]
    pub fn should_log(&mut self) -> bool {
        let now = Instant::now();
        self.suppressed_count += 1;

        let should_log = match self.last_log_time {
            None => true,  // 第一次总是输出
            Some(last_time) => {
                // 达到数量阈值 或 时间阈值
                self.suppressed_count >= self.count_interval
                    || now.duration_since(last_time) >= self.time_interval
            }
        };

        if should_log {
            self.last_log_time = Some(now);
            self.suppressed_count = 0;
        }

        should_log
    }

    /// 获取被抑制的日志数量
    #[allow(dead_code)]
    pub fn suppressed_count(&self) -> u64 {
        self.suppressed_count
    }

    /// 重置计数器
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.suppressed_count = 0;
        self.last_log_time = None;
    }
}

/// 停滞检测器
pub struct StagnationDetector {
    last_activity: Instant,
    warning_threshold: Duration,
    critical_threshold: Duration,
    check_interval: Duration,
    last_check: Instant,
}

impl StagnationDetector {
    /// 创建新的停滞检测器
    pub fn new(
        warning_secs: u64,
        critical_secs: u64,
        check_interval_secs: u64,
    ) -> Self {
        Self {
            last_activity: Instant::now(),
            warning_threshold: Duration::from_secs(warning_secs),
            critical_threshold: Duration::from_secs(critical_secs),
            check_interval: Duration::from_secs(check_interval_secs),
            last_check: Instant::now(),
        }
    }

    /// 记录活动
    pub fn mark_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// 检查是否停滞
    pub fn check_stagnation(&mut self) -> StagnationStatus {
        let now = Instant::now();
        
        // 检查间隔控制
        if now.duration_since(self.last_check) < self.check_interval {
            return StagnationStatus::Normal;
        }
        
        self.last_check = now;
        
        let idle_time = now.duration_since(self.last_activity);

        if idle_time >= self.critical_threshold {
            StagnationStatus::Critical(idle_time.as_secs())
        } else if idle_time >= self.warning_threshold {
            StagnationStatus::Warning(idle_time.as_secs())
        } else {
            StagnationStatus::Normal
        }
    }

    /// 获取空闲时间（秒）
    #[allow(dead_code)]
    pub fn idle_time_secs(&self) -> u64 {
        self.last_activity.elapsed().as_secs()
    }
}

/// 停滞状态
#[derive(Debug, Clone, PartialEq)]
pub enum StagnationStatus {
    Normal,
    Warning(u64),   // 警告，空闲秒数
    Critical(u64),  // 严重，空闲秒数
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut buffer = RingBuffer::new(3);
        
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        
        assert_eq!(buffer.len(), 3);
        
        // 继续push会覆盖最旧的
        buffer.push(4);
        assert_eq!(buffer.len(), 3);
        
        let vec = buffer.to_vec();
        assert_eq!(vec, vec![2, 3, 4]);
    }

    #[test]
    fn test_ring_buffer_empty() {
        let buffer: RingBuffer<i32> = RingBuffer::new(5);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.to_vec().len(), 0);
    }

    #[test]
    fn test_log_throttler() {
        let mut throttler = LogThrottler::new(3, 1000);
        
        // 第一次应该输出
        assert!(throttler.should_log());
        
        // 接下来2次被抑制
        assert!(!throttler.should_log());
        assert!(!throttler.should_log());
        
        // 第3次应该输出（达到数量阈值）
        assert!(throttler.should_log());
    }

    #[test]
    fn test_stagnation_detector() {
        let mut detector = StagnationDetector::new(2, 5, 1);
        
        // 立即检查应该是正常的
        assert_eq!(detector.check_stagnation(), StagnationStatus::Normal);
        
        // 等待后检查
        std::thread::sleep(Duration::from_secs(3));
        let status = detector.check_stagnation();
        assert!(matches!(status, StagnationStatus::Warning(_)));
    }
}
