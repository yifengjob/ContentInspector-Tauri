import {invoke} from '@tauri-apps/api/core'
import {listen, UnlistenFn} from '@tauri-apps/api/event'
import type {AppConfig, DirectoryNode, ScanConfig, ScanResultItem} from '@/types'

// ==================== 基础 API ====================

// 获取目录树
export async function getDirectoryTree(path: string, showHidden = true): Promise<DirectoryNode[]> {
  // Tauri命令参数默认使用camelCase
  return await invoke('get_directory_tree', { path, showHidden })
}

// 开始扫描
export async function startScan(config: ScanConfig): Promise<void> {
  // 【优化】后端已配置 #[serde(rename_all = "camelCase")]，直接传递 camelCase 格式
  return await invoke('scan_start', { config })
}

// 取消扫描
export async function cancelScan(): Promise<boolean> {
  return await invoke('scan_cancel')
}

// ==================== 预览 API ====================

// 流式预览文件
export async function previewFileStream(filePath: string): Promise<{ success: boolean; total_chunks?: number }> {
  console.log('[previewFileStream] 🚀 调用 Tauri invoke, path:', filePath);
  const result = await invoke<{ success: boolean; total_chunks?: number }>('preview_file_stream', { path: filePath });
  console.log('[previewFileStream] ✅ 返回结果:', result);
  return result;
}

// 监听预览数据块
export async function onPreviewChunk(callback: (chunk: any) => void): Promise<UnlistenFn> {
  console.log('[onPreviewChunk] 🎧 开始注册 preview-chunk 监听器');
  const unlisten = await listen('preview-chunk', (event) => {
    console.log('[onPreviewChunk] ✅ 收到事件, payload:', event.payload);
    callback(event.payload)
  });
  console.log('[onPreviewChunk] ✅ 监听器注册完成');
  return unlisten;
}

// 【新增】监听预览错误
export async function onPreviewError(callback: (error: string) => void): Promise<UnlistenFn> {
  return await listen('preview-error', (event) => {
    callback(event.payload as string)
  })
}

// 取消预览
export async function cancelPreview(taskId?: number): Promise<boolean> {
  if (!taskId) return false
  return await invoke('cancel_preview', { task_id: taskId })
}

// ==================== 文件操作 API ====================

// 打开文件
export async function openFile(filePath: string): Promise<void> {
  return await invoke('open_file', { path: filePath })
}

// 打开文件所在目录
export async function openFileLocation(filePath: string): Promise<void> {
  return await invoke('open_file_location', { path: filePath })
}

// 删除文件
export async function deleteFile(filePath: string, toTrash: boolean = false): Promise<void> {
  return await invoke('delete_file', { path: filePath, to_trash: toTrash })
}

// ==================== 报告导出 API ====================

// 导出报告
export async function exportReport(
  results: ScanResultItem[],
  format: 'csv' | 'json' | 'excel',
  filePath?: string
): Promise<void> {
  // 转换format名称
  const rustFormat = format === 'excel' ? 'xlsx' : format
  return await invoke('export_report', { 
    results, 
    format: rustFormat, 
    save_path: filePath || '' 
  })
}

// ==================== 日志和规则 API ====================

// 获取日志
export async function getLogs(): Promise<string[]> {
  return await invoke('get_logs')
}

// 获取敏感规则
export async function getSensitiveRules(): Promise<Array<[string, string, boolean]>> {
  return await invoke('get_sensitive_rules')
}

// ==================== 配置 API ====================

// 保存配置
export async function saveConfig(config: AppConfig): Promise<void> {
  // 【优化】后端已配置 #[serde(rename_all = "camelCase")]，直接传递 camelCase 格式
  return await invoke('save_config', { config })
}

// 加载配置
export async function loadConfig(): Promise<AppConfig> {
  return await invoke('load_config')
}

// 获取推荐的并发数
export async function getRecommendedConcurrency(): Promise<number> {
  const result = await invoke('get_recommended_concurrency') as {
    recommended: number
    max_allowed: number
    cpu_count: number
    free_memory_gb: string
  }
  return result.recommended
}

// ==================== 环境检查 API ====================

// 检查系统环境
export async function checkSystemEnvironment(): Promise<any> {
  const result = await invoke('check_system_environment')
  console.log('[环境检查] 后端返回结果:', result)
  return result
}

// 清理缓存
export async function clearCache(
  options?: {
    cleanLogs?: boolean
    cleanTemp?: boolean
    logRetentionDays?: number
  }
): Promise<{
  success: boolean
  directories_cleaned?: number
  files_cleaned?: number
  space_freed_bytes?: number
  space_freed_formatted?: string
  details?: string[]
}> {
  return await invoke('clear_cache', {
    cleanLogs: options?.cleanLogs ?? true,
    cleanTemp: options?.cleanTemp ?? true,
    logRetentionDays: options?.logRetentionDays ?? 30,
  })
}

// ==================== 事件监听 API ====================

// 监听扫描进度事件
export async function onScanProgress(callback: (data: any) => void): Promise<UnlistenFn> {
  return await listen('scan-progress', (event) => {
    callback(event.payload)
  })
}

// 监听扫描结果事件（支持批量模式）
export async function onScanResult(
  callback: (data: ScanResultItem | ScanResultItem[]) => void,
  batchMode: boolean = false
): Promise<UnlistenFn> {
  if (batchMode) {
    // 使用批量事件
    return await listen('scan-batch-result', (event) => {
      callback(event.payload as ScanResultItem[])
    })
  } else {
    // 使用单个事件
    return await listen('scan-result', (event) => {
      callback(event.payload as ScanResultItem)
    })
  }
}

// 监听扫描完成事件
export async function onScanFinished(callback: () => void): Promise<UnlistenFn> {
  return await listen('scan-finished', () => {
    callback()
  })
}

// 监听扫描错误事件
export async function onScanError(callback: (error: string) => void): Promise<UnlistenFn> {
  return await listen('scan-error', (event) => {
    callback(event.payload as string)
  })
}

// 监听扫描日志事件
export async function onScanLog(callback: (log: string) => void): Promise<UnlistenFn> {
  return await listen('scan-log', (event) => {
    callback(event.payload as string)
  })
}

// 监听批量日志事件（带节流）
export async function onScanLogBatch(callback: (logs: string[]) => void): Promise<UnlistenFn> {
  // Tauri中没有原生的批量日志，使用单条日志模拟
  let buffer: string[] = []
  let timer: number | null = null
  
  const flushBuffer = () => {
    if (buffer.length > 0) {
      callback([...buffer])
      buffer = []
    }
  }
  
  return await listen('scan-log', (event) => {
    buffer.push(event.payload as string)
    
    // 清除之前的定时器
    if (timer !== null) {
      clearTimeout(timer)
    }
    
    // 设置新的定时器（100ms后flush）
    timer = window.setTimeout(flushBuffer, 100)
  })
}

// ==================== 对话框 API ====================

// 显示消息对话框
export async function showMessage(
  message: string,
  options?: {
    title?: string
    type?: 'info' | 'warning' | 'error'
  }
): Promise<void> {
  const { message: dialogMessage } = await import('@tauri-apps/plugin-dialog')
  await dialogMessage(message, {
    title: options?.title || '提示',
    kind: options?.type === 'warning' ? 'warning' : options?.type === 'error' ? 'error' : 'info',
    buttons: { ok: '确定'}
  })
}

// 确认对话框（类似 confirm）
export async function askDialog(
  message: string,
  options?: {
    title?: string
    type?: 'info' | 'warning' | 'error' | 'question'
    okLabel?: string
    cancelLabel?: string
  }
): Promise<boolean> {
  const { ask } = await import('@tauri-apps/plugin-dialog')
  return await ask(message, {
    title: options?.title || '确认',
    kind: options?.type === 'warning' ? 'warning' : options?.type === 'error' ? 'error' : 'info',
    okLabel: options?.okLabel || '确定',
    cancelLabel: options?.cancelLabel || '取消',
  })
}

// 保存文件对话框
export async function showSaveDialog(options?: {
  filters?: Array<{ name: string; extensions: string[] }>
}): Promise<string | null> {
  const { save } = await import('@tauri-apps/plugin-dialog')
  const filePath = await save({
    filters: options?.filters,
  })
  return filePath || null
}

// ==================== 开发者工具 API ====================

// 打开开发者工具（仅debug模式）
export async function openDevTools(): Promise<void> {
  try {
    await invoke('open_dev_tools')
  } catch (error) {
    console.warn('打开开发者工具失败:', error)
  }
}

// ==================== 搜索表达式 API ====================

// 设置搜索表达式
export async function setSearchExpression(expression: string): Promise<void> {
  return await invoke('set_search_expression', { expression: expression || null })
}

// 获取当前搜索表达式
export async function getSearchExpression(): Promise<string> {
  const result = await invoke('get_search_expression') as string | null
  return result || ''
}

// 验证表达式语法（用于前端实时校验）
export async function validateExpression(
  expression: string
): Promise<{ valid: boolean; error?: string; position?: number }> {
  return await invoke('validate_search_expression', { expression })
}

// ==================== 【补充】批量扫描结果监听 ====================

// 监听批量扫描结果事件
export async function onScanBatchResult(callback: (data: ScanResultItem[]) => void): Promise<UnlistenFn> {
  return await listen('scan-batch-result', (event) => {
    callback(event.payload as ScanResultItem[])
  })
}
