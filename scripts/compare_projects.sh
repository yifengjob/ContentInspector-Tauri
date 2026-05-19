#!/bin/bash
# 深度对比分析脚本 - 生成详细的文件清单

echo "=== Electron版文件清单 ==="
find /Users/yifeng/数据/开发/项目/ElectronProjects/DataGuardScanner/src -type f -name "*.ts" | sort > /tmp/electron_backend_files.txt
find /Users/yifeng/数据/开发/项目/ElectronProjects/DataGuardScanner/frontend/src -type f \( -name "*.vue" -o -name "*.ts" \) | sort > /tmp/electron_frontend_files.txt

echo "=== Tauri版文件清单 ==="
find /Users/yifeng/数据/开发/项目/RustroverProjects/DataGuardScanner/src-tauri/src -type f -name "*.rs" | sort > /tmp/tauri_backend_files.txt
find /Users/yifeng/数据/开发/项目/RustroverProjects/DataGuardScanner/frontend/src -type f \( -name "*.vue" -o -name "*.ts" \) | sort > /tmp/tauri_frontend_files.txt

echo "Electron后端文件数: $(wc -l < /tmp/electron_backend_files.txt)"
echo "Electron前端文件数: $(wc -l < /tmp/electron_frontend_files.txt)"
echo "Tauri后端文件数: $(wc -l < /tmp/tauri_backend_files.txt)"
echo "Tauri前端文件数: $(wc -l < /tmp/tauri_frontend_files.txt)"

echo ""
echo "=== 文件清单已生成 ==="
echo "Electron后端: /tmp/electron_backend_files.txt"
echo "Electron前端: /tmp/electron_frontend_files.txt"
echo "Tauri后端: /tmp/tauri_backend_files.txt"
echo "Tauri前端: /tmp/tauri_frontend_files.txt"
