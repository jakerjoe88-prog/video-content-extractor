import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface GpuInfo {
  available: boolean;
  name?: string;
  memory_mb?: number;
  cuda_version?: string;
  driver_version?: string;
  device_id: number; // DirectML 设备索引
  vendor?: string;
  architecture?: string;
  is_discrete: boolean;
}

// 后端返回的完整检测结果
export interface GpuDetectionInfo {
  devices: GpuInfo[];
  recommended_device?: number;
  gpu_available: boolean;
  fallback_reason?: string;
  compatibility_report?: string;
}

export interface GpuConfig {
  enabled: boolean;
  device_id: number; // GPU 设备 ID
  threads: number;
  memory_limit_mb: number;
  batch_size: number;
}

export interface RecommendedConfig {
  threads: number;
  memory_limit_mb: number;
  batch_size: number;
  reason: string;
}

interface GpuStore {
  gpuInfo: GpuInfo | null; // 当前选中的 GPU
  devices: GpuInfo[]; // 所有检测到的 GPU 设备
  recommendedDeviceIndex: number | null; // 推荐的设备索引
  config: GpuConfig;
  recommendedConfig: RecommendedConfig | null;
  isLoading: boolean;
  compatibilityReport: string | null;
  
  // Actions
  detectGpu: () => Promise<void>;
  selectDevice: (deviceIndex: number) => void; // 选择 GPU 设备
  updateConfig: (config: Partial<GpuConfig>) => void;
  getRecommendedConfig: () => Promise<void>;
  applyRecommendedConfig: () => void;
  resetToDefault: () => void;
  validateConfig: () => Promise<boolean>;
}

const defaultConfig: GpuConfig = {
  enabled: true,
  device_id: 0,
  threads: 4,
  memory_limit_mb: 2048,
  batch_size: 1,
};

export const useGpuStore = create<GpuStore>((set, get) => ({
  gpuInfo: null,
  devices: [],
  recommendedDeviceIndex: null,
  config: defaultConfig,
  recommendedConfig: null,
  isLoading: false,
  compatibilityReport: null,

  detectGpu: async () => {
    set({ isLoading: true });
    try {
      // 先获取用户已保存的设置
      let savedDeviceId: number | null = null;
      try {
        const settings = await invoke<{ gpu_device_id: number }>("get_settings");
        savedDeviceId = settings.gpu_device_id;
      } catch (e) {
        console.warn("获取已保存的 GPU 设置失败:", e);
      }
      
      const result = await invoke<GpuDetectionInfo>("detect_gpu_info");
      
      // 存储所有设备
      const devices = result.devices || [];
      const recommendedIndex = result.recommended_device ?? null;
      
      // 选择 GPU 设备的优先级：
      // 1. 用户已保存的设备（如果存在）
      // 2. 推荐的设备
      // 3. 第一个可用设备
      let selectedGpu: GpuInfo | null = null;
      let selectedDeviceId = 0;
      
      if (devices.length > 0) {
        // 优先使用用户已保存的设备
        if (savedDeviceId !== null) {
          const savedDevice = devices.find(d => d.device_id === savedDeviceId);
          if (savedDevice) {
            selectedGpu = savedDevice;
            selectedDeviceId = savedDevice.device_id;
            console.log("使用已保存的 GPU 设备:", savedDevice.name, "device_id:", savedDeviceId);
          }
        }
        
        // 如果没有找到已保存的设备，使用推荐的设备
        if (!selectedGpu) {
          if (recommendedIndex !== null && recommendedIndex < devices.length) {
            selectedGpu = devices[recommendedIndex];
            selectedDeviceId = selectedGpu.device_id;
            console.log("使用推荐的 GPU 设备:", selectedGpu.name, "device_id:", selectedDeviceId);
          } else {
            selectedGpu = devices[0];
            selectedDeviceId = selectedGpu.device_id;
            console.log("使用第一个 GPU 设备:", selectedGpu.name, "device_id:", selectedDeviceId);
          }
        }
      }
      
      set({ 
        devices,
        gpuInfo: selectedGpu,
        recommendedDeviceIndex: recommendedIndex,
        compatibilityReport: result.compatibility_report || null,
      });
      
      // 更新 config 中的 device_id
      set((state) => ({
        config: { ...state.config, device_id: selectedDeviceId },
      }));
      
      // 自动获取推荐配置
      await get().getRecommendedConfig();
    } catch (error) {
      console.error("GPU 检测失败:", error);
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },

  selectDevice: (deviceIndex: number) => {
    const { devices } = get();
    if (deviceIndex >= 0 && deviceIndex < devices.length) {
      const selectedGpu = devices[deviceIndex];
      set({ 
        gpuInfo: selectedGpu,
        config: { ...get().config, device_id: selectedGpu.device_id },
      });
      // 重新获取推荐配置
      get().getRecommendedConfig();
      
      // 保存设备选择到后端设置
      invoke("save_gpu_device_id", { deviceId: selectedGpu.device_id })
        .catch((error) => {
          console.error("保存 GPU 设备 ID 失败:", error);
        });
    }
  },

  updateConfig: (newConfig: Partial<GpuConfig>) => {
    set((state) => ({
      config: { ...state.config, ...newConfig },
    }));
  },

  getRecommendedConfig: async () => {
    const { gpuInfo } = get();
    if (!gpuInfo) return;

    try {
      const recommended = await invoke<RecommendedConfig>(
        "get_recommended_gpu_config",
        { gpuInfo }
      );
      set({ recommendedConfig: recommended });
    } catch (error) {
      console.error("获取推荐配置失败:", error);
    }
  },

  applyRecommendedConfig: () => {
    const { recommendedConfig, gpuInfo } = get();
    if (!recommendedConfig) return;

    set({
      config: {
        enabled: true,
        device_id: gpuInfo?.device_id ?? 0,
        threads: recommendedConfig.threads,
        memory_limit_mb: recommendedConfig.memory_limit_mb,
        batch_size: recommendedConfig.batch_size,
      },
    });
  },

  resetToDefault: () => {
    const { gpuInfo } = get();
    set({ 
      config: {
        ...defaultConfig,
        device_id: gpuInfo?.device_id ?? 0,
      }
    });
  },

  validateConfig: async () => {
    const { config, gpuInfo } = get();
    if (!gpuInfo) return false;

    try {
      const isValid = await invoke<boolean>("validate_gpu_config", {
        config,
        gpuInfo,
      });
      return isValid;
    } catch (error) {
      console.error("配置验证失败:", error);
      throw error;
    }
  },
}));
