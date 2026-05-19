/// <reference types="vite/client" />

// SVG 模块类型声明
declare module '*.svg' {
  const content: string;
  export default content;
}

// SVG Icons 虚拟模块声明（vite-plugin-svg-icons）
declare module 'virtual:svg-icons-register' {
  const _default: void;
  export default _default;
}

// Tauri 平台检测
declare global {
  interface Window {
    __TAURI__?: any;
  }
}

export {};
