/**
 * 文件读取工具函数
 */

import { invoke } from '@tauri-apps/api/core';
import type { IpcResponse } from '@/types/preview';

/**
 * 文件读取结果类型
 */
type FileReadResult = IpcResponse<ArrayBuffer>;
/**
 * 读取文件为 ArrayBuffer
 * @param filePath 文件路径
 * @returns Promise<FileReadResult>
 */
export async function readFileAsBlob(filePath: string): Promise<FileReadResult> {
  try {
    // 【修复】使用 Tauri API 而不是 Electron API
    // 注意：Tauri 后端返回 Vec<u8>，前端接收为 Uint8Array
    const data = await invoke<Uint8Array | ArrayBuffer>('read_file_as_blob', { path: filePath });
    
    // 将 Uint8Array 或 ArrayBuffer 统一转换为 ArrayBuffer
    let arrayBuffer: ArrayBuffer;
    if (data instanceof Uint8Array) {
      // Uint8Array 需要提取其底层的 ArrayBuffer
      // 使用 as ArrayBuffer 进行类型断言，因为 SharedArrayBuffer 在此场景不适用
      arrayBuffer = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
    } else if (data instanceof ArrayBuffer) {
      // 已经是 ArrayBuffer，直接使用
      arrayBuffer = data;
    } else {
      // 其他情况（如 number[]），转换为 Uint8Array 再获取 buffer
      arrayBuffer = new Uint8Array(data as unknown as number[]).buffer;
    }
    
    return {
      success: true,
      data: arrayBuffer,
    };
  } catch (_error) {
    const errorMessage = _error instanceof Error ? _error.message : '读取文件失败';
    console.error('[readFileAsBlob] 错误:', errorMessage);
    return {
      success: false,
      error: errorMessage,
    };
  }
}
