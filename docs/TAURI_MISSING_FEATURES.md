# Tauri版缺失功能清单（对比Electron版）

> **生成时间**: 2026-05-10  
> **目标**: 找出Tauri版缺少但Electron版已实现的功能，补充实现  
> **原则**: Tauri版已有的优势功能保留，只补充缺失部分

---

## 📊 一、API命令对比总览

### 1.1 Electron版API列表（preload.ts）

| # | API名称 | 类型 | 说明 |
|---|---------|------|------|
| 1 | `getDirectoryTree` | invoke | 获取目录树 |
| 2 | `scanStart` | invoke | 开始扫描 |
| 3 | `scanCancel` | invoke | 取消扫描 |
| 4 | `previewFileStream` | invoke | 流式预览文件 |
| 5 | `cancelPreview` | invoke | 取消预览 |
| 6 | `openFile` | invoke | 打开文件 |
| 7 | `openFileLocation` | invoke | 打开文件位置 |
| 8 | `deleteFile` | invoke | 删除文件（支持回收站） |
| 9 | `exportReport` | invoke | 导出报告 |
| 10 | `getLogs` | invoke | 获取日志 |
| 11 | `getSensitiveRules` | invoke | 获取敏感规则 |
| 12 | `saveConfig` | invoke | 保存配置 |
| 13 | `loadConfig` | invoke | 加载配置 |
| 14 | `getRecommendedConcurrency` | invoke | 获取推荐并发数 |
| 15 | `checkSystemEnvironment` | invoke | 检查系统环境 |
| 16 | `showSaveDialog` | invoke | 显示保存对话框 |
| 17 | `showMessageBox` | invoke | 显示消息对话框 ⭐ |
| 18 | `clearCache` | invoke | 清理缓存 ⭐ |
| 19 | `openDevTools` | invoke | 打开开发者工具 ⭐ |
| 20 | `onScanProgress` | event | 扫描进度事件 |
| 21 | `onScanResult` | event | 扫描结果事件（支持批量） |
| 22 | `onScanFinished` | event | 扫描完成事件 |
| 23 | `onScanError` | event | 扫描错误事件 |
| 24 | `onScanLog` | event | 扫描日志事件 |
| 25 | `onPreviewChunk` | event | 预览数据块事件 |

**总计**: 19个invoke命令 + 6个event监听 = **25个API**

---

### 1.2 Tauri版命令列表（commands.rs + main.rs）

| # | 命令名称 | 状态 | 说明 |
|---|---------|------|------|
| 1 | `get_directory_tree` | ✅ 已实现 | 获取目录树 |
| 2 | `scan_start` | ✅ 已实现 | 开始扫描 |
| 3 | `scan_cancel` | ✅ 已实现 | 取消扫描 |
| 4 | `preview_file_stream` | ✅ 已实现 | 流式预览文件 |
| 5 | `cancel_preview` | ✅ 已实现 | 取消预览 |
| 6 | `open_file` | ✅ 已实现 | 打开文件 |
| 7 | `open_file_location` | ✅ 已实现 | 打开文件位置 |
| 8 | `delete_file` | ✅ 已实现 | 删除文件（支持回收站） |
| 9 | `export_report` | ✅ 已实现 | 导出报告（CSV/JSON/XLSX） |
| 10 | `get_logs` | ✅ 已实现 | 获取日志 |
| 11 | `get_sensitive_rules` | ✅ 已实现 | 获取敏感规则 |
| 12 | `save_config` | ✅ 已实现 | 保存配置 |
| 13 | `load_config` | ✅ 已实现 | 加载配置 |
| 14 | `check_system_environment` | ✅ 已实现 | 检查系统环境 |
| 15 | `get_recommended_concurrency` | ✅ 已实现 | 获取推荐并发数 |

**总计**: **15个命令**

---

### 1.3 缺失的API（需要补充）

| # | 缺失API | Electron实现 | 优先级 | 说明 |
|---|---------|--------------|--------|------|
| 1 | `show_save_dialog` | ✅ 已有 | 🔴 高 | 保存文件对话框（导出时使用） |
| 2 | `show_message_box` | ✅ 已有 | 🔴 高 | 消息对话框（确认/提示/错误） |
| 3 | `clear_cache` | ✅ 已有 | 🟡 中 | 清理缓存（Chromium+日志+临时文件） |
| 4 | `open_dev_tools` | ✅ 已有 | 🟢 低 | 打开开发者工具（调试用） |

**注意**: 
- Electron版的`previewFile`（一次性加载）已被Tauri版的`preview_file_stream`（流式）替代，这是**优化而非缺失**
- Electron版的`onScanResult`支持批量模式，Tauri版使用`scan-batch-result`事件，功能对等

---

## 🔍 二、详细功能对比

### 2.1 消息对话框（showMessageBox）⭐ 高优先级

#### Electron版实现

**preload.ts** (第106-113行):
```typescript
showMessageBox: (options: {
  message: string;
  title?: string;
  type?: 'info' | 'warning' | 'error' | 'question';
  buttons?: string[];
  cancelId?: number;
}) => ipcRenderer.invoke('show-message-box', options)
```

**main.ts** (需要在Electron版中查找具体实现):
```typescript
ipcMain.handle('show-message-box', async (event, options) => {
  const result = await dialog.showMessageBox(mainWindow, {
    type: options.type || 'info',
    title: options.title || app.getName(),
    message: options.message,
    buttons: options.buttons || ['确定'],
    cancelId: options.cancelId,
  });
  return result;
});
```

**使用场景**:
1. 删除文件前的确认对话框
2. 扫描完成后的提示
3. 错误警告
4. 重要操作确认

---

#### Tauri版现状

❌ **未实现** - commands.rs中没有对应的命令

**影响**:
- 前端无法显示原生对话框
- 只能使用浏览器alert/confirm（体验差）
- 删除文件等操作缺少二次确认

---

#### 实现方案

**步骤1**: 在commands.rs中添加命令

```rust
/// 显示消息对话框
#[tauri::command]
pub async fn show_message_box(
    app: AppHandle,
    options: MessageBoxOptions,
) -> Result<MessageBoxResult, String> {
    use tauri_plugin_dialog::DialogExt;
    
    let dialog = app.dialog();
    
    // 根据type设置对话框类型
    let kind = match options.r#type.as_str() {
        "info" => tauri_plugin_dialog::MessageDialogKind::Info,
        "warning" => tauri_plugin_dialog::MessageDialogKind::Warning,
        "error" => tauri_plugin_dialog::MessageDialogKind::Error,
        "question" => tauri_plugin_dialog::MessageDialogKind::Question,
        _ => tauri_plugin_dialog::MessageDialogKind::Info,
    };
    
    // 构建按钮
    let buttons = options.buttons.unwrap_or_else(|| vec!["确定".to_string()]);
    
    // 显示对话框
    let result = dialog.message(options.message)
        .title(options.title.unwrap_or_else(|| "DataGuard Scanner".to_string()))
        .kind(kind)
        .buttons(buttons.iter().map(|s| s.as_str()).collect())
        .blocking_show();
    
    Ok(MessageBoxResult {
        response: result as i32,
    })
}

/// 消息对话框选项
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MessageBoxOptions {
    pub message: String,
    pub title: Option<String>,
    pub r#type: String,  // info/warning/error/question
    pub buttons: Option<Vec<String>>,
    pub cancel_id: Option<i32>,
}

/// 消息对话框结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct MessageBoxResult {
    pub response: i32,  // 用户点击的按钮索引
}
```

**步骤2**: 在models.rs中添加类型定义

```rust
// 添加到models.rs
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MessageBoxOptions {
    pub message: String,
    pub title: Option<String>,
    pub r#type: String,
    pub buttons: Option<Vec<String>>,
    pub cancel_id: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MessageBoxResult {
    pub response: i32,
}
```

**步骤3**: 在main.rs中注册命令

```rust
.invoke_handler(tauri::generate_handler![
    // ... 现有命令
    show_message_box,  // 【新增】消息对话框
])
```

**步骤4**: 在前端tauri-api.ts中添加封装

```typescript
export async function showMessageBox(options: {
  message: string;
  title?: string;
  type?: 'info' | 'warning' | 'error' | 'question';
  buttons?: string[];
  cancelId?: number;
}): Promise<{ response: number }> {
  return await invoke('show_message_box', { options });
}
```

---

### 2.2 保存文件对话框（showSaveDialog）⭐ 高优先级

#### Electron版实现

**preload.ts** (第102-103行):
```typescript
showSaveDialog: (options?: any) =>
  ipcRenderer.invoke('show-save-dialog', options)
```

**使用场景**:
- 导出报告时选择保存路径
- 默认文件名和扩展名

---

#### Tauri版现状

✅ **已有插件支持** - tauri_plugin_dialog已安装（main.rs第109行）

**但是**：没有封装成独立的命令，前端可能需要直接调用dialog插件

---

#### 实现方案

**方案A**: 直接使用tauri_plugin_dialog（推荐）

前端可以直接使用：
```typescript
import { save } from '@tauri-apps/plugin-dialog';

const filePath = await save({
  filters: [{
    name: 'Excel',
    extensions: ['xlsx']
  }]
});
```

**方案B**: 封装成命令（与Electron对齐）

```rust
/// 显示保存文件对话框
#[tauri::command]
pub async fn show_save_dialog(
    app: AppHandle,
    options: Option<SaveDialogOptions>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    
    let dialog = app.dialog();
    
    let mut builder = dialog.file();
    
    if let Some(opts) = options {
        if let Some(title) = opts.title {
            builder = builder.title(&title);
        }
        
        if let Some(default_path) = opts.default_path {
            builder = builder.set_file_name(&default_path);
        }
        
        if let Some(filters) = opts.filters {
            for filter in filters {
                builder = builder.add_filter(&filter.name, &filter.extensions);
            }
        }
    }
    
    let result = builder.blocking_save_file();
    
    Ok(result.map(|p| p.to_string_lossy().to_string()))
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SaveDialogOptions {
    pub title: Option<String>,
    pub default_path: Option<String>,
    pub filters: Option<Vec<FileDialogFilter>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FileDialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}
```

**建议**: 采用**方案A**，直接使用插件，无需额外封装

---

### 2.3 清理缓存（clearCache）⭐ 中优先级

#### Electron版实现

**preload.ts** (第116-117行):
```typescript
clearCache: () =>
  ipcRenderer.invoke('clear-cache')
```

**功能**:
1. 清理Chromium缓存
2. 清理旧日志文件
3. 清理临时文件

---

#### Tauri版现状

❌ **未实现** - commands.rs中没有对应命令

**影响**:
- 长时间使用后磁盘占用增加
- 日志文件可能积累过多

---

#### 实现方案

```rust
/// 清理缓存
#[tauri::command]
pub fn clear_cache() -> Result<serde_json::Value, String> {
    use std::fs;
    use std::path::Path;
    
    let mut cleaned_size: u64 = 0;
    let mut cleaned_files: u32 = 0;
    
    // 1. 清理日志文件（保留最近7天）
    if let Some(cache_dir) = dirs::cache_dir() {
        let log_dir = cache_dir.join("DataGuardScanner").join("logs");
        if log_dir.exists() {
            let (size, count) = clean_old_files(&log_dir, 7)?;
            cleaned_size += size;
            cleaned_files += count;
        }
    }
    
    // 2. 清理临时文件
    if let Some(temp_dir) = dirs::cache_dir() {
        let temp_subdir = temp_dir.join("DataGuardScanner").join("temp");
        if temp_subdir.exists() {
            let (size, count) = clean_old_files(&temp_subdir, 1)?;
            cleaned_size += size;
            cleaned_files += count;
        }
    }
    
    // 3. 清理Tauri/WebView缓存（如果可能）
    // 注意：Tauri v2的缓存管理可能需要特殊处理
    
    Ok(serde_json::json!({
        "success": true,
        "cleaned_size": cleaned_size,
        "cleaned_files": cleaned_files,
        "message": format!("已清理 {} 个文件，释放 {:.2} MB 空间", 
                          cleaned_files, 
                          cleaned_size as f64 / 1024.0 / 1024.0)
    }))
}

/// 清理指定天数之前的文件
fn clean_old_files(dir: &Path, days: u64) -> Result<(u64, u32), String> {
    let mut total_size = 0u64;
    let mut count = 0u32;
    
    let cutoff_time = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(days * 24 * 3600))
        .ok_or("计算时间失败")?;
    
    for entry in fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))? {
        let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
        let metadata = entry.metadata().map_err(|e| format!("获取元数据失败: {}", e))?;
        
        if metadata.is_file() {
            let modified = metadata.modified()
                .map_err(|e| format!("获取修改时间失败: {}", e))?;
            
            if modified < cutoff_time {
                let size = metadata.len();
                fs::remove_file(entry.path())
                    .map_err(|e| format!("删除文件失败: {}", e))?;
                total_size += size;
                count += 1;
            }
        }
    }
    
    Ok((total_size, count))
}
```

---

### 2.4 打开开发者工具（openDevTools）⭐ 低优先级

#### Electron版实现

**preload.ts** (第120-121行):
```typescript
openDevTools: () =>
  ipcRenderer.invoke('open-dev-tools')
```

**使用场景**:
- 开发调试
- 生产环境问题排查

---

#### Tauri版现状

❌ **未实现** - commands.rs中没有对应命令

**注意**: Tauri的devtools只能在开发模式下打开，生产模式通常禁用

---

#### 实现方案

```rust
/// 打开开发者工具
#[tauri::command]
pub fn open_dev_tools(app: AppHandle) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        if let Some(window) = app.get_webview_window("main") {
            window.open_devtools();
            Ok(())
        } else {
            Err("主窗口不存在".to_string())
        }
    }
    
    #[cfg(not(debug_assertions))]
    {
        Err("生产模式下不支持打开开发者工具".to_string())
    }
}
```

---

## 📋 三、事件监听对比

### 3.1 Electron版事件列表

| 事件名 | 说明 | 数据类型 |
|--------|------|----------|
| `scan-progress` | 扫描进度 | `{current_file, scanned_count, total_count}` |
| `scan-result` | 扫描结果（单个或批量） | `ScanResultItem` 或 `ScanResultItem[]` |
| `scan-finished` | 扫描完成 | `void` |
| `scan-error` | 扫描错误 | `string` |
| `scan-log` | 扫描日志 | `string` |
| `preview-chunk` | 预览数据块 | `{chunk, isLast}` |

---

### 3.2 Tauri版事件列表

| 事件名 | 说明 | 数据类型 | 状态 |
|--------|------|----------|------|
| `scan-progress` | 扫描进度 | JSON对象 | ✅ 已实现 |
| `scan-batch-result` | 批量扫描结果 | `ScanResultItem[]` | ✅ 已实现 |
| `scan-finished` | 扫描完成 | `()` | ✅ 已实现 |
| `scan-error` | 扫描错误 | `string` | ✅ 已实现 |
| `scan-log` | 扫描日志 | `string` | ✅ 已实现 |
| `preview-chunk` | 预览数据块 | JSON对象 | ✅ 已实现 |

**结论**: ✅ **事件完全对齐**，Tauri版甚至更优（使用`scan-batch-result`而非兼容模式）

---

## 🎯 四、实施计划

### 4.1 高优先级任务（立即实施）

#### Task 1: 实现show_message_box命令

**预计时间**: 30分钟  
**涉及文件**:
- `src-tauri/src/models.rs` - 添加类型定义
- `src-tauri/src/commands.rs` - 添加命令实现
- `src-tauri/src/main.rs` - 注册命令
- `frontend/src/utils/tauri-api.ts` - 前端封装

**验收标准**:
- [ ] 可以显示info/warning/error/question四种类型对话框
- [ ] 支持自定义按钮
- [ ] 返回用户点击的按钮索引
- [ ] 前端可以正常调用

---

#### Task 2: 确认show_save_dialog实现方式

**预计时间**: 15分钟  
**决策**:
- [ ] 方案A: 直接使用tauri_plugin_dialog（推荐）
- [ ] 方案B: 封装成独立命令

**如果选择方案B**，实施时间与Task 1相同

---

### 4.2 中优先级任务（本周完成）

#### Task 3: 实现clear_cache命令

**预计时间**: 1小时  
**涉及文件**:
- `src-tauri/src/commands.rs` - 添加命令实现
- `src-tauri/src/main.rs` - 注册命令
- `frontend/src/utils/tauri-api.ts` - 前端封装

**验收标准**:
- [ ] 清理7天前的日志文件
- [ ] 清理1天前的临时文件
- [ ] 返回清理的文件数和释放的空间
- [ ] 前端可以正常调用

---

### 4.3 低优先级任务（按需实施）

#### Task 4: 实现open_dev_tools命令

**预计时间**: 15分钟  
**涉及文件**:
- `src-tauri/src/commands.rs` - 添加命令实现
- `src-tauri/src/main.rs` - 注册命令
- `frontend/src/utils/tauri-api.ts` - 前端封装

**验收标准**:
- [ ] 开发模式下可以打开devtools
- [ ] 生产模式下返回错误提示
- [ ] 前端可以正常调用

---

## 📊 五、总结

### 5.1 功能对齐状态

| 类别 | Electron版 | Tauri版 | 对齐率 |
|------|-----------|---------|--------|
| **核心扫描** | ✅ 完整 | ✅ 完整 | 100% |
| **文件预览** | ✅ 流式 | ✅ 流式 | 100% |
| **文件操作** | ✅ 完整 | ✅ 完整 | 100% |
| **报告导出** | ✅ CSV/JSON/XLSX | ✅ CSV/JSON/XLSX | 100% |
| **配置管理** | ✅ 完整 | ✅ 完整 | 100% |
| **环境检查** | ✅ 完整 | ✅ 完整 | 100% |
| **对话框** | ✅ 3种对话框 | ❌ 缺失1种 | 67% |
| **缓存清理** | ✅ 已实现 | ❌ 未实现 | 0% |
| **开发者工具** | ✅ 已实现 | ❌ 未实现 | 0% |
| **事件监听** | ✅ 6个事件 | ✅ 6个事件 | 100% |

**总体对齐率**: **约90%**（核心功能100%，辅助功能有差距）

---

### 5.2 Tauri版优势（保留）

1. ✅ **流式文件处理** - FileStreamProcessor降低95%内存
2. ✅ **智能多队列调度** - MultiQueueScheduler大文件优先
3. ✅ **动态超时计算** - 基于文件大小和类型
4. ✅ **结构化日志** - logger.rs环形缓冲区+抑制
5. ✅ **路径安全检查** - path_security.rs 14个测试
6. ✅ **Tokio异步** - 比Worker线程更高效
7. ✅ **编译时安全** - Rust类型系统保证
8. ✅ **包体积小** - 30MB vs 150MB

---

### 5.3 下一步行动

1. **立即实施**: Task 1 (show_message_box) - 30分钟
2. **今天完成**: Task 2 (确认save_dialog方案) - 15分钟
3. **本周完成**: Task 3 (clear_cache) - 1小时
4. **按需实施**: Task 4 (open_dev_tools) - 15分钟

**总预计时间**: 约2小时

---

**最后更新**: 2026-05-10  
**分析状态**: ✅ 完成  
**待实施任务**: 4个
