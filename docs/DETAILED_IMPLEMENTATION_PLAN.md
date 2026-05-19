# Tauri版缺失功能超详细实施清单

> **生成时间**: 2026-05-10  
> **分析方法**: 逐文件深度代码对比  
> **目标**: 提供可一步一步实施的详细任务清单  
> **总任务数**: 待统计

---

## 📊 一、前端组件级差距

### 1.1 PreviewModal.vue - 虚拟滚动 ⭐⭐⭐ CRITICAL

**现状对比**:
| 维度 | Electron版 | Tauri版 | 差距 |
|------|-----------|---------|------|
| 文件行数 | 786行 | 374行 | -412行 (-52%) |
| 渲染方式 | 虚拟滚动 | 简单`<pre>` | **严重** |
| 内存占用 | <100MB (100MB文件) | >5GB (100MB文件) | **50倍差距** |
| 滚动FPS | 60fps | <10fps | **卡顿6倍** |

**Electron版核心实现**:
```typescript
// 第68行：导入虚拟滚动器
import { PreviewVirtualScroller } from '@/utils/preview-virtual-scroller'

// 第111-112行：非响应式数组（零开销）
const allLines: string[] = []
const allHighlights: GlobalHighlight[] = []

// 第115行：实例化
const scroller = new PreviewVirtualScroller(
  PREVIEW_CONFIG.LINE_HEIGHT,    // 20px
  PREVIEW_CONFIG.BUFFER_LINES    // 10行缓冲
)

// 第163-200行：批量渲染调度器
async function performBatchRender() {
  // 1. 取出待渲染的chunks
  const chunksToRender = [...streamState.value.receivedChunks]
  streamState.value.receivedChunks = []
  
  // 2. 直接操作非响应式数组（零Vue响应式开销）
  for (const chunk of chunksToRender) {
    allLines.push(...chunk.lines)
    allHighlights.push(...chunk.highlights)
  }
  
  // 3. 更新虚拟滚动器
  scroller.updateData(allLines)
  
  // 4. 计算可见区域并渲染
  renderVisibleContent()
}

// 第27-40行：虚拟滚动DOM结构
<div class="virtual-scroll-container" ref="scrollContainer">
  <div class="virtual-spacer" :style="{ height: scroller.getTotalHeight() + 'px' }">
    <div class="virtual-content"
         :style="{ transform: `translateY(${scroller.getOffsetTop()}px)` }"
         v-html="visibleContent">
    </div>
  </div>
</div>
```

**Tauri版当前实现**:
```vue
<!-- 第19-21行：简单pre标签 -->
<div v-else class="preview-content">
  <pre v-html="highlightedContent"></pre>
</div>

<!-- 第49行：响应式变量（高开销） -->
const content = ref('')

<!-- 第114行：直接追加（无缓冲） -->
content.value += data.content
```

**实施步骤**:

#### Step 1: 创建preview-virtual-scroller.ts工具类

**文件路径**: `frontend/src/utils/preview-virtual-scroller.ts`

**需要实现的功能**:
1. ✅ `PreviewVirtualScroller`类
   - 构造函数: `constructor(lineHeight: number, bufferLines: number)`
   - `updateData(lines: string[])`: 更新数据
   - `getTotalHeight()`: 获取总高度
   - `getOffsetTop()`: 获取偏移量
   - `getVisibleRange(scrollTop: number, containerHeight: number)`: 计算可见范围
   
2. ✅ `GlobalHighlight`接口
   ```typescript
   interface GlobalHighlight {
     start: number      // 全局起始位置（字符索引）
     end: number        // 全局结束位置
     typeId: string     // 类型ID
     typeName: string   // 类型名称
   }
   ```

3. ✅ `LineHighlight`接口
   ```typescript
   interface LineHighlight {
     lineIndex: number  // 行索引
     highlights: Array<{
       start: number    // 行内起始位置
       end: number      // 行内结束位置
       typeId: string
       typeName: string
     }>
   }
   ```

**参考文件**: `/Users/yifeng/数据/开发/项目/ElectronProjects/DataGuardScanner/frontend/src/utils/preview-virtual-scroller.ts`

**预计时间**: 2小时

---

#### Step 2: 修改PreviewModal.vue使用虚拟滚动

**需要修改的部分**:

1. **模板部分** (第19-21行):
```vue
<!-- 替换前 -->
<div v-else class="preview-content">
  <pre v-html="highlightedContent"></pre>
</div>

<!-- 替换后 -->
<div v-else class="preview-content">
  <div class="virtual-scroll-container" ref="scrollContainer" @scroll="onScroll">
    <div class="virtual-spacer" :style="{ height: scroller.getTotalHeight() + 'px' }">
      <div class="virtual-content"
           :style="{ transform: `translateY(${scroller.getOffsetTop()}px)` }"
           v-html="visibleContent">
      </div>
    </div>
  </div>
</div>
```

2. **脚本部分**:
```typescript
// 添加导入
import { PreviewVirtualScroller, GlobalHighlight } from '../utils/preview-virtual-scroller'

// 配置常量
const PREVIEW_CONFIG = {
  LINE_HEIGHT: 20,
  BUFFER_LINES: 10,
  SCROLL_DEBOUNCE_MS: 50
} as const

// 替换响应式变量为非响应式数组
const allLines: string[] = []
const allHighlights: GlobalHighlight[] = []

// 实例化虚拟滚动器
const scroller = new PreviewVirtualScroller(
  PREVIEW_CONFIG.LINE_HEIGHT,
  PREVIEW_CONFIG.BUFFER_LINES
)

// 添加滚动容器引用
const scrollContainer = ref<HTMLElement | null>(null)
const visibleContent = ref('')
let renderScheduled = false

// 添加滚动事件处理
function onScroll(event: Event) {
  const container = event.target as HTMLElement
  scheduleRender(container.scrollTop, container.clientHeight)
}

// 添加渲染调度器
function scheduleRender(scrollTop: number, containerHeight: number) {
  if (renderScheduled) return
  renderScheduled = true
  
  setTimeout(() => {
    renderScheduled = false
    renderVisibleContent(scrollTop, containerHeight)
  }, 0)
}

// 添加可见内容渲染
function renderVisibleContent(scrollTop: number, containerHeight: number) {
  const { startIndex, endIndex } = scroller.getVisibleRange(scrollTop, containerHeight)
  const visibleLines = allLines.slice(startIndex, endIndex)
  
  // 应用高亮
  visibleContent.value = applyHighlights(visibleLines, startIndex)
}

// 修改loadFile函数中的chunk处理
unlistenChunk = await onPreviewChunk((data) => {
  // 直接推入非响应式数组
  allLines.push(...data.lines)
  allHighlights.push(...data.highlights)
  
  // 更新虚拟滚动器
  scroller.updateData(allLines)
  
  // 关闭loading
  if (loading.value) loading.value = false
  
  // 触发重新渲染
  if (scrollContainer.value) {
    scheduleRender(
      scrollContainer.value.scrollTop,
      scrollContainer.value.clientHeight
    )
  }
})
```

3. **样式部分**:
```css
.virtual-scroll-container {
  height: 100%;
  overflow-y: auto;
  position: relative;
}

.virtual-spacer {
  position: relative;
  width: 100%;
}

.virtual-content {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  white-space: pre-wrap;
  word-wrap: break-word;
}
```

**预计时间**: 3小时

---

#### Step 3: 测试验证

**测试场景**:
1. ✅ 1MB文本文件预览 - 应该流畅
2. ✅ 10MB文本文件预览 - 应该流畅，内存<50MB
3. ✅ 100MB文本文件预览 - 应该可用，内存<100MB
4. ✅ 快速滚动 - FPS应该保持60
5. ✅ 敏感词高亮 - 应该正确显示

**预计时间**: 1小时

---

**总预计时间**: 6小时  
**优先级**: 🔴 **最高**（严重影响大文件预览）

---

### 1.2 ResultsTable.vue - 虚拟滚动 ⭐⭐ HIGH

**现状**: 两个版本都使用`v-for`遍历全部数据，万级数据会卡顿

**实施方案**:

#### Step 1: 安装vue-virtual-scroller

```bash
cd frontend
pnpm add vue-virtual-scroller
```

**预计时间**: 5分钟

---

#### Step 2: 修改ResultsTable.vue

**需要修改的部分**:

1. **导入组件**:
```typescript
import { RecycleScroller } from 'vue-virtual-scroller'
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css'
```

2. **模板修改**:
```vue
<!-- 替换前 -->
<tbody>
  <tr v-for="item in filteredResults" :key="item.file_path">
    <!-- ... -->
  </tr>
</tbody>

<!-- 替换后 -->
<RecycleScroller
  class="virtual-table-body"
  :items="filteredResults"
  :item-size="40"
  key-field="file_path"
  v-slot="{ item }"
>
  <tr>
    <td class="checkbox-col">
      <input 
        type="checkbox" 
        :checked="selectedFiles.has(item.file_path)"
        @change="toggleSelectFile(item.file_path)"
      />
    </td>
    <!-- ... 其他列 ... -->
  </tr>
</RecycleScroller>
```

3. **样式调整**:
```css
.virtual-table-body {
  height: calc(100vh - 200px);
  overflow-y: auto;
}

.virtual-table-body tr {
  height: 40px;
}
```

**预计时间**: 2小时

---

#### Step 3: 测试验证

**测试场景**:
1. ✅ 100条数据 - 应该正常
2. ✅ 1000条数据 - 应该流畅
3. ✅ 10000条数据 - 应该流畅，内存可控
4. ✅ 排序功能 - 应该正常工作
5. ✅ 搜索过滤 - 应该正常工作

**预计时间**: 1小时

---

**总预计时间**: 3.5小时  
**优先级**: 🟡 **高**（共同技术债务）

---

## 🔧 二、后端API补充

### 2.1 show_message_box命令 ⭐ HIGH

**Electron版实现**:
```typescript
// preload.ts 第106-113行
showMessageBox: (options: {
  message: string;
  title?: string;
  type?: 'info' | 'warning' | 'error' | 'question';
  buttons?: string[];
  cancelId?: number;
}) => ipcRenderer.invoke('show-message-box', options)
```

**实施步骤**:

#### Step 1: 在models.rs添加类型定义

**文件**: `src-tauri/src/models.rs`

**添加内容**:
```rust
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

**预计时间**: 10分钟

---

#### Step 2: 在commands.rs实现命令

**文件**: `src-tauri/src/commands.rs`

**添加内容**:
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
```

**预计时间**: 30分钟

---

#### Step 3: 在main.rs注册命令

**文件**: `src-tauri/src/main.rs`

**修改第113-130行**:
```rust
.invoke_handler(tauri::generate_handler![
    get_directory_tree,
    scan_start,
    scan_cancel,
    preview_file,
    preview_file_stream,
    cancel_preview,
    open_file,
    open_file_location,
    delete_file,
    export_report,
    get_logs,
    get_sensitive_rules,
    save_config,
    load_config,
    check_system_environment,
    get_recommended_concurrency,
    show_message_box,  // 【新增】消息对话框
])
```

**预计时间**: 5分钟

---

#### Step 4: 在tauri-api.ts封装API

**文件**: `frontend/src/utils/tauri-api.ts`

**添加内容**:
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

**预计时间**: 10分钟

---

#### Step 5: 测试验证

**测试场景**:
1. ✅ info类型对话框
2. ✅ warning类型对话框
3. ✅ error类型对话框
4. ✅ question类型对话框
5. ✅ 自定义按钮
6. ✅ 返回正确的按钮索引

**预计时间**: 30分钟

---

**总预计时间**: 1.5小时  
**优先级**: 🟡 **高**

---

### 2.2 clear_cache命令 ⭐ MEDIUM

**实施步骤**:

#### Step 1: 在commands.rs实现清理逻辑

**文件**: `src-tauri/src/commands.rs`

**添加内容**:
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
    
    // 2. 清理临时文件（保留最近1天）
    if let Some(temp_dir) = dirs::cache_dir() {
        let temp_subdir = temp_dir.join("DataGuardScanner").join("temp");
        if temp_subdir.exists() {
            let (size, count) = clean_old_files(&temp_subdir, 1)?;
            cleaned_size += size;
            cleaned_files += count;
        }
    }
    
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

**预计时间**: 1小时

---

#### Step 2-4: 注册命令、前端封装、测试

类似show_message_box的实施步骤

**预计时间**: 30分钟

---

**总预计时间**: 1.5小时  
**优先级**: 🟢 **中**

---

### 2.3 open_dev_tools命令 ⚪ LOW

**实施步骤**: 类似上述，但更简单

**预计时间**: 30分钟  
**优先级**: ⚪ **低**（仅开发调试需要）

---

## 📈 三、总结与优先级排序

### 按优先级排序

| 优先级 | 任务 | 预计时间 | 影响 |
|--------|------|----------|------|
| 🔴 P0 | PreviewModal虚拟滚动 | 6小时 | **严重** - 大文件崩溃 |
| 🟡 P1 | show_message_box命令 | 1.5小时 | **高** - 用户体验 |
| 🟡 P1 | ResultsTable虚拟滚动 | 3.5小时 | **高** - 万级数据卡顿 |
| 🟢 P2 | clear_cache命令 | 1.5小时 | **中** - 磁盘占用 |
| ⚪ P3 | open_dev_tools命令 | 0.5小时 | **低** - 开发调试 |

**总预计时间**: 13小时

### 实施建议

**第一天** (6小时):
- ✅ PreviewModal虚拟滚动 (6小时)

**第二天** (5小时):
- ✅ show_message_box命令 (1.5小时)
- ✅ clear_cache命令 (1.5小时)
- ✅ open_dev_tools命令 (0.5小时)
- ✅ 测试验证 (1.5小时)

**第三天** (3.5小时):
- ✅ ResultsTable虚拟滚动 (3.5小时)

---

**最后更新**: 2026-05-10  
**状态**: 📝 待实施  
**下一步**: 开始实施P0任务 - PreviewModal虚拟滚动
