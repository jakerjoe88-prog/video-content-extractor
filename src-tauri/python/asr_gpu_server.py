"""
GPU 加速语音识别服务
使用 sherpa-onnx Python 绑定 + SenseVoice 模型
支持 DirectML GPU 加速 (Windows)
"""

import os
import sys
import time
import wave
import struct
from pathlib import Path
from typing import Optional
import uvicorn
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

app = FastAPI(title="ASR GPU Server")

# 全局模型实例
recognizer = None
current_device = "cpu"
current_num_threads = 4
model_loaded = False

class TranscribeResponse(BaseModel):
    status: str
    text: Optional[str] = None
    duration_ms: Optional[int] = None
    device: Optional[str] = None
    rtf: Optional[float] = None
    error: Optional[str] = None

class StatusResponse(BaseModel):
    status: str
    model_loaded: bool
    device: str
    gpu_available: bool
    gpu_name: Optional[str] = None

class LoadRequest(BaseModel):
    use_gpu: bool = True
    gpu_device_id: int = 0

def get_models_dir() -> Path:
    """获取模型目录"""
    if "ASR_MODELS_DIR" in os.environ:
        return Path(os.environ["ASR_MODELS_DIR"])
    
    if sys.platform == "win32":
        # 使用 Roaming AppData 目录
        app_data = Path(os.environ.get("APPDATA", ""))
        return app_data / "DouyinCreatorToolkit" / "models" / "asr" / "sense-voice"
    else:
        return Path.home() / ".local" / "share" / "DouyinCreatorToolkit" / "models" / "asr" / "sense-voice"

def detect_gpu() -> tuple:
    """检测可用的 GPU，优先返回 NVIDIA/AMD 独立显卡"""
    try:
        if sys.platform == "win32":
            import subprocess
            result = subprocess.run(
                ["wmic", "path", "win32_VideoController", "get", "name"],
                capture_output=True, text=True
            )
            lines = [l.strip() for l in result.stdout.split('\n') if l.strip() and l.strip() != "Name"]
            
            if lines:
                # 优先选择 NVIDIA 或 AMD 独立显卡
                for i, gpu in enumerate(lines):
                    if "NVIDIA" in gpu.upper() or "AMD" in gpu.upper() or "RADEON" in gpu.upper():
                        print(f"[ASR-GPU] 检测到独立显卡: {gpu} (索引: {i})")
                        return True, gpu, i
                # 如果没有独立显卡，返回第一个
                return True, lines[0], 0
    except Exception as e:
        print(f"[ASR-GPU] GPU 检测失败: {e}")
    
    return False, None, 0

def read_wav_samples(audio_path: str) -> tuple:
    """读取 WAV 文件并返回采样数据"""
    with wave.open(audio_path, 'rb') as wav:
        sample_rate = wav.getframerate()
        n_frames = wav.getnframes()
        n_channels = wav.getnchannels()
        sample_width = wav.getsampwidth()
        
        raw_data = wav.readframes(n_frames)
        
        # 转换为浮点数
        if sample_width == 2:
            fmt = f"<{n_frames * n_channels}h"
            samples = list(struct.unpack(fmt, raw_data))
            samples = [s / 32768.0 for s in samples]
        elif sample_width == 4:
            fmt = f"<{n_frames * n_channels}i"
            samples = list(struct.unpack(fmt, raw_data))
            samples = [s / 2147483648.0 for s in samples]
        else:
            raise ValueError(f"不支持的采样位深: {sample_width * 8}")
        
        # 转换为单声道
        if n_channels == 2:
            samples = [(samples[i] + samples[i+1]) / 2 for i in range(0, len(samples), 2)]
        
        duration_ms = int(len(samples) / sample_rate * 1000)
        return samples, sample_rate, duration_ms

def get_best_directml_device_id(models_dir: Path) -> int:
    """
    智能探测最佳 DirectML 设备 ID。
    策略：优先尝试 ID 1 (笔记本独显)，失败则回退 ID 0 (台式机/集显)，均失败则回退 CPU (-1)。
    """
    import sherpa_onnx
    
    # 构造一个最小化的探测配置
    model_path = str(models_dir / "model.onnx")
    tokens_path = str(models_dir / "tokens.txt")
    
    # 如果没有模型文件，无法探测
    if not (models_dir / "model.onnx").exists():
        print(f"[ASR-GPU] 探测失败: 模型文件不存在 {model_path}")
        return 0 # 默认回退到 0，后续流程会报错

    def probe_device(device_id):
        """发送一个'探针'，看设备是否能响应"""
        try:
            print(f"[ASR-GPU] 🔍 正在探测 DirectML 设备 ID: {device_id}...")
            
            # 使用最简配置，debug=False 减少日志
            provider = f"dml:{device_id}"
            
            # 使用 from_sense_voice 工厂方法
            _ = sherpa_onnx.OfflineRecognizer.from_sense_voice(
                model=model_path,
                tokens=tokens_path,
                num_threads=1,
                provider=provider,
                language="auto",
                use_itn=False,
                debug=False,
            )
            
            print(f"[ASR-GPU] ✅ 设备 ID {device_id} 初始化成功！")
            return True
        except Exception as e:
            # 简化错误日志，取第一行
            err_msg = str(e).split('\n')[0]
            print(f"[ASR-GPU] ❌ 设备 ID {device_id} 不可用。原因: {err_msg}...")
            return False

    # === 核心策略 ===
    
    # 1. 优先探测 ID 1
    # 理由：在 Windows 双显卡笔记本上，ID 1 几乎 100% 是独立显卡（NVIDIA/AMD）。
    if probe_device(1):
        print("[ASR-GPU] 🚀 选中策略：高性能独立显卡 (ID: 1)")
        return 1
        
    # 2. 回退探测 ID 0
    # 理由：如果 ID 1 不存在，说明用户是单显卡环境（台式机独显 或 笔记本纯集显）。
    if probe_device(0):
        print("[ASR-GPU] 🚗 选中策略：默认显示适配器 (ID: 0)")
        return 0
        
    # 3. 彻底失败
    print("[ASR-GPU] ⚠️ 未检测到支持 DirectML 的 GPU，将回退到 CPU。")
    return -1

def load_model(use_gpu: bool = True, gpu_device_id: int = 0, num_threads: int = 4) -> bool:
    """加载 SenseVoice 模型"""
    global recognizer, current_device, model_loaded, current_num_threads
    
    current_num_threads = num_threads
    
    try:
        import sherpa_onnx
        
        models_dir = get_models_dir()
        print(f"[ASR-GPU] 模型目录: {models_dir}")
        
        # 检查模型文件
        model_file = models_dir / "model.onnx"
        model_int8 = models_dir / "model.int8.onnx"
        tokens_file = models_dir / "tokens.txt"
        
        # 优先使用非量化模型 (GPU 友好)
        if model_file.exists():
            actual_model = str(model_file)
        elif model_int8.exists():
            actual_model = str(model_int8)
        else:
            print(f"[ASR-GPU] 模型文件不存在: {model_file}")
            return False
        
        if not tokens_file.exists():
            print(f"[ASR-GPU] tokens 文件不存在: {tokens_file}")
            return False
        
        # 智能选择 provider：尝试 CUDA，失败回退 CPU
        provider = "cpu"
        current_device = "CPU"
        # use passed num_threads
        
        if use_gpu:
            # 检查是否有 NVIDIA GPU
            gpu_available, gpu_name, _ = detect_gpu()
            if gpu_available and "nvidia" in gpu_name.lower():
                print(f"[ASR-GPU] 检测到 NVIDIA GPU: {gpu_name}，尝试 CUDA 加速...")
                try:
                    # 先尝试用 CUDA 加载
                    test_recognizer = sherpa_onnx.OfflineRecognizer.from_sense_voice(
                        model=actual_model,
                        tokens=str(tokens_file),
                        num_threads=4,
                        provider="cuda",
                        language="auto",
                        use_itn=True,
                        debug=False,
                    )
                    # 成功了！
                    provider = "cuda"
                    current_device = "GPU (CUDA)"
                    # num_threads for CUDA is typically handled internally or set to a low value
                    # but we keep user value or force to smaller if needed?
                    # for now, keep as is or let user decide. Actually sherpa might ignore it for CUDA.
                    print("[ASR-GPU] ✅ CUDA 加速已启用！")
                    # 直接使用这个 recognizer
                    recognizer = test_recognizer
                except Exception as e:
                    print(f"[ASR-GPU] ⚠️ CUDA 初始化失败: {e}")
                    import traceback
                    traceback.print_exc()
                    print("[ASR-GPU] 回退到 CPU 模式")
            else:
                print(f"[ASR-GPU] 未检测到 NVIDIA GPU (当前: {gpu_name})，使用 CPU 模式")
        
        print(f"[ASR-GPU] 最终配置: Provider={provider}, Threads={num_threads}")
        
        start_time = time.time()
        
        # 如果还没创建 recognizer（没走 CUDA 分支或 CUDA 失败）
        if provider == "cpu":
            recognizer = sherpa_onnx.OfflineRecognizer.from_sense_voice(
                model=actual_model,
                tokens=str(tokens_file),
                num_threads=num_threads,
                provider=provider,
                language="auto",
                use_itn=True,
                debug=True,
            )
        
        load_time = time.time() - start_time
        print(f"[ASR-GPU] 模型加载完成，耗时: {load_time:.2f}s")
        print(f"[ASR-GPU] 当前设备: {current_device}")
        
        model_loaded = True
        return True
        
    except Exception as e:
        print(f"[ASR-GPU] 模型加载失败: {e}")
        import traceback
        traceback.print_exc()
        current_device = "Error"
        return False

@app.get("/health")
async def health_check():
    """健康检查"""
    return {"status": "ok", "service": "asr-gpu"}

@app.get("/status")
async def get_status() -> StatusResponse:
    """获取服务状态"""
    gpu_available, gpu_name, _ = detect_gpu()
    return StatusResponse(
        status="ok",
        model_loaded=model_loaded,
        device=current_device,
        gpu_available=gpu_available,
        gpu_name=gpu_name
    )

@app.post("/load")
async def load_model_endpoint(request: LoadRequest = None):
    """加载模型"""
    use_gpu = request.use_gpu if request else True
    gpu_device_id = request.gpu_device_id if request else 0
    
    success = load_model(use_gpu=use_gpu, gpu_device_id=gpu_device_id)
    if success:
        return {"status": "ok", "message": "模型加载成功", "device": current_device}
    else:
        raise HTTPException(status_code=500, detail="模型加载失败")

@app.post("/unload")
async def unload_model():
    """卸载模型"""
    global recognizer, model_loaded
    recognizer = None
    model_loaded = False
    return {"status": "ok", "message": "模型已卸载"}

@app.post("/transcribe")
async def transcribe(request: TranscribeRequest) -> TranscribeResponse:
    """转写音频文件"""
    global recognizer, current_device, model_loaded
    
    audio_path = request.audio_path
    if not os.path.exists(audio_path):
        return TranscribeResponse(
            status="error",
            error=f"音频文件不存在: {audio_path}"
        )
    
    
    # 如果请求的模式和当前模式不同，或者线程数变了，重新加载模型
    requested_gpu = request.use_gpu
    requested_threads = request.num_threads or 4
    current_is_gpu = "GPU" in current_device
    
    if recognizer is None or requested_gpu != current_is_gpu or (requested_gpu is False and requested_threads != current_num_threads):
        print(f"[ASR-GPU] 配置变更: GPU={requested_gpu}, Threads={requested_threads}")
        if not load_model(use_gpu=requested_gpu, num_threads=requested_threads):
            return TranscribeResponse(
                status="error",
                error="模型加载失败"
            )
    
    try:
        print(f"[ASR-GPU] 开始转写: {audio_path}")
        start_time = time.time()
        
        # 读取音频
        samples, sample_rate, duration_ms = read_wav_samples(audio_path)
        
        # 创建流并转写
        stream = recognizer.create_stream()
        stream.accept_waveform(sample_rate, samples)
        recognizer.decode_stream(stream)
        
        text = stream.result.text
        
        transcribe_time = time.time() - start_time
        rtf = transcribe_time / (duration_ms / 1000) if duration_ms > 0 else 0
        
        print(f"[ASR-GPU] 转写完成:")
        print(f"  音频时长: {duration_ms/1000:.2f}s")
        print(f"  转写耗时: {transcribe_time:.2f}s")
        print(f"  实时率 (RTF): {rtf:.3f}x")
        print(f"  设备: {current_device}")
        
        return TranscribeResponse(
            status="success",
            text=text.strip(),
            duration_ms=duration_ms,
            device=current_device,
            rtf=rtf
        )
        
    except Exception as e:
        print(f"[ASR-GPU] 转写失败: {e}")
        import traceback
        traceback.print_exc()
        return TranscribeResponse(
            status="error",
            error=str(e)
        )

# 配置
ASR_PORT = int(os.environ.get("ASR_GPU_PORT", "38081"))
ASR_HOST = os.environ.get("ASR_GPU_HOST", "127.0.0.1")

if __name__ == "__main__":
    print("[ASR-GPU] 启动 GPU 语音识别服务...")
    print(f"[ASR-GPU] 服务地址: http://{ASR_HOST}:{ASR_PORT}")
    
    # 启动时自动加载模型 (GPU 模式)
    load_model(use_gpu=True, gpu_device_id=0)
    
    uvicorn.run(app, host=ASR_HOST, port=ASR_PORT)
