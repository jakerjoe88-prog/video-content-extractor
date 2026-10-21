import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": "/src",
    },
  },
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    host: "127.0.0.1",
    watch: {
      ignored: ["**/src-tauri/**"],
    },
    // 预热常用文件
    warmup: {
      clientFiles: [
        "./src/App.tsx",
        "./src/main.tsx",
        "./src/components/layout/MainLayout.tsx",
        "./src/pages/LocalVideo.tsx",
      ],
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-react': ['react', 'react-dom'],
          'vendor-ui': ['@radix-ui/react-dialog', '@radix-ui/react-tabs', '@radix-ui/react-toast'],
        },
      },
    },
    minify: 'esbuild',
    chunkSizeWarningLimit: 500,
  },
  // 预构建所有常用依赖，避免运行时转换
  optimizeDeps: {
    include: [
      'react',
      'react-dom',
      'zustand',
      'clsx',
      'tailwind-merge',
      'class-variance-authority',
      'lucide-react',
      '@radix-ui/react-dialog',
      '@radix-ui/react-tabs',
      '@radix-ui/react-toast',
      '@radix-ui/react-slot',
      '@radix-ui/react-switch',
      '@radix-ui/react-progress',
      '@radix-ui/react-select',
      '@radix-ui/react-dropdown-menu',
      '@radix-ui/react-scroll-area',
      '@tauri-apps/api',
      '@tauri-apps/plugin-dialog',
      '@tauri-apps/plugin-fs',
    ],
    // 强制预构建，不等待首次访问
    force: false,
  },
  // 缓存目录
  cacheDir: 'node_modules/.vite',
});
